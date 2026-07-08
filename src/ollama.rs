use crate::config::Config;
use crate::modelfile::parse_modelfile_content;
use regex::Regex;
use reqwest::Client;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub size: String,
    pub modified_at: String,
    pub digest: String,
}

#[derive(Debug, Clone, Default)]
pub struct RunningModel {
    pub name: String,
    pub size: String,
    pub digest: String,
    pub expires_at: String,
    pub size_vram: u64,
}

pub struct OllamaClient {
    client: Client,
    config: Config,
}

impl OllamaClient {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();

        Self { client, config }
    }

    pub fn base_url(&self) -> &str {
        &self.config.ollama_base_url
    }

    pub async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url());
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await?;
        let mut models = vec![];
        if let Some(arr) = body.get("models").and_then(|m| m.as_array()) {
            for m in arr {
                models.push(ModelInfo {
                    name: m.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
                    size: m.get("size").and_then(|v| v.as_str()).unwrap_or("").into(),
                    modified_at: m
                        .get("modified_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    digest: m
                        .get("digest")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                });
            }
        }
        Ok(models)
    }

    pub async fn list_running_models(&self) -> anyhow::Result<Vec<RunningModel>> {
        let url = format!("{}/api/ps", self.base_url());
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await?;
        let mut models = vec![];
        if let Some(arr) = body.get("models").and_then(|m| m.as_array()) {
            for m in arr {
                models.push(RunningModel {
                    name: m.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
                    size: m.get("size").and_then(|v| v.as_str()).unwrap_or("").into(),
                    digest: m
                        .get("digest")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    expires_at: m
                        .get("expires_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    size_vram: m.get("size_vram").and_then(|v| v.as_u64()).unwrap_or(0),
                });
            }
        }
        Ok(models)
    }

    pub async fn pull_model(&self, model_name: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/pull", self.base_url());
        let payload = serde_json::json!({ "name": model_name, "stream": true });
        info!("Pulling model: {}", model_name);
        let resp = self.client.post(&url).json(&payload).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to pull model: {}", resp.status()));
        }

        Ok(())
    }

    pub async fn create_model(
        &self,
        model_name: &str,
        modelfile_path: &PathBuf,
    ) -> anyhow::Result<()> {
        let modelfile_content = std::fs::read_to_string(modelfile_path)?;
        let base_model = extract_base_model(&modelfile_content);

        if let Some(base) = base_model {
            info!("Ensuring base model '{}' exists...", base);
            let existing = self.list_models().await?;
            if !existing.iter().any(|m| m.name == base) {
                self.pull_model(&base).await?;
            }
        }

        let existing = self.list_models().await?;
        if existing.iter().any(|m| m.name == model_name) {
            info!(
                "Model '{}' exists, removing before recreation...",
                model_name
            );
            let _ = self.remove_model(model_name).await;
        }

        let parsed_modelfile = parse_modelfile_content(&modelfile_content)?;
        let mut payload = serde_json::json!({
            "name": model_name,
            "stream": true,
        });

        if let Some(from) = parsed_modelfile.from {
            payload["from"] = serde_json::Value::String(from);
        }

        if let Some(system) = parsed_modelfile.system {
            payload["system"] = serde_json::Value::String(system);
        }

        if !parsed_modelfile.parameters.is_empty() {
            let mut params = serde_json::Map::new();
            let mut stops: Vec<String> = Vec::new();
            for param in &parsed_modelfile.parameters {
                if param.key == "stop" {
                    stops.push(param.value.clone());
                } else {
                    params.insert(
                        param.key.clone(),
                        serde_json::Value::String(param.value.clone()),
                    );
                }
            }
            if !stops.is_empty() {
                params.insert(
                    "stop".to_string(),
                    serde_json::Value::Array(
                        stops.into_iter().map(serde_json::Value::String).collect(),
                    ),
                );
            }
            payload["parameters"] = serde_json::Value::Object(params);
        }

        if let Some(template) = parsed_modelfile.template {
            payload["template"] = serde_json::Value::String(template);
        }

        let url = format!("{}/api/create", self.base_url());

        info!(
            "Creating model '{}' from modelfile: {:?}",
            model_name, modelfile_path
        );
        let resp = self.client.post(&url).json(&payload).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to create model from modelfile: {} - {}",
                status,
                text
            ));
        }

        info!("Model '{}' created successfully", model_name);
        Ok(())
    }

    pub async fn remove_model(&self, model_name: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/delete", self.base_url());
        let payload = serde_json::json!({ "name": model_name });
        self.client.delete(&url).json(&payload).send().await?;
        Ok(())
    }

    pub async fn ensure_model(
        &self,
        model_name: &str,
        modelfile_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let existing = self.list_models().await?;
        let already_present = existing.iter().any(|m| m.name == model_name);

        if already_present && modelfile_name.is_none() {
            info!("Model '{}' already present.", model_name);
            return Ok(());
        }

        if modelfile_name.is_some() {
            info!(
                "Model '{}' present, recreating from updated modfile...",
                model_name
            );
        } else {
            info!("Model '{}' not found, pulling...", model_name);
        }

        if let Some(mf_name) = modelfile_name {
            let mf_path = self.config.modfile_dir.join(mf_name);
            if mf_path.exists() {
                if let Err(e) = self.create_model(model_name, &mf_path).await {
                    warn!(
                        "Failed to create from modfile: {}, falling back to registry pull",
                        e
                    );
                    self.pull_model(model_name).await?;
                }
                return Ok(());
            }
            warn!("Modfile not found at {:?}", mf_path);
        }

        self.pull_model(model_name).await
    }

    pub async fn warmup_model(&self, model_name: &str, timeout_seconds: u64) -> anyhow::Result<()> {
        info!("Warming up model: {}", model_name);
        let prompt = if model_name.contains("30b")
            || model_name.contains("26b")
            || model_name.contains("27b")
        {
            "Hello".to_string()
        } else {
            "ok".to_string()
        };

        let url = format!("{}/api/generate", self.base_url());
        let payload = serde_json::json!({
            "model": model_name,
            "prompt": prompt,
            "stream": false,
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .timeout(Duration::from_secs(timeout_seconds))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Warmup for '{}' failed with status {}",
                model_name,
                resp.status()
            ));
        }

        info!("Model '{}' warmed up successfully", model_name);
        Ok(())
    }

    pub async fn stop_model(&self, model_name: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/generate", self.base_url());
        let payload = serde_json::json!({
            "model": model_name,
            "prompt": "",
            "keep_alive": "0",
        });
        self.client.post(&url).json(&payload).send().await?;
        Ok(())
    }
}

pub fn extract_base_model(modelfile_content: &str) -> Option<String> {
    let re = Regex::new(r"(?i)^FROM\s+(\S+)\s*$").unwrap();
    for line in modelfile_content.lines() {
        let line = line.trim();
        if let Some(caps) = re.captures(line) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }
    None
}

pub fn ollama_running(port: u16) -> bool {
    if let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        if let Ok(resp) = client
            .get(format!("http://127.0.0.1:{}/api/tags", port))
            .send()
        {
            return resp.status().is_success();
        }
    }
    false
}

pub fn start_ollama_direct(config: &Config, log_file: &PathBuf) -> anyhow::Result<(u32, bool)> {
    let mut cmd = Command::new(&config.ollama_bin);
    cmd.arg("serve");

    cmd.env("OLLAMA_HOST", &config.ollama_host);
    cmd.env("OLLAMA_PORT", config.ollama_port.to_string());
    cmd.env(
        "OLLAMA_NUM_PARALLEL",
        config.ollama_num_parallel.to_string(),
    );
    cmd.env(
        "OLLAMA_MAX_LOADED_MODELS",
        config.ollama_max_loaded_models.to_string(),
    );

    if config.platform.is_macos() {
        cmd.env(
            "OLLAMA_FLASH_ATTENTION",
            config.ollama_flash_attention.unwrap_or(1).to_string(),
        );
        cmd.env("OLLAMA_KV_CACHE_TYPE", &config.ollama_kv_cache_type);
    } else if matches!(
        config.platform,
        crate::config::Platform::CachyOS | crate::config::Platform::Linux
    ) {
        cmd.env("CUDA_VISIBLE_DEVICES", "0");
        cmd.env("OLLAMA_KV_CACHE_TYPE", &config.ollama_kv_cache_type);
        if let Some(gpu_layers) = config.ollama_gpu_layers {
            cmd.env("OLLAMA_GPU_LAYERS", gpu_layers.to_string());
        }
    }

    if let Some(ref models_path) = config.ollama_models_path {
        cmd.env("OLLAMA_MODELS", models_path.to_str().unwrap_or_default());
    }

    use std::fs::OpenOptions;
    std::fs::create_dir_all(
        log_file
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
    )?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.stderr(file.try_clone()?);
        cmd.stdout(file);
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    let mut child = cmd.spawn()?;
    let pid = child.id();
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    std::thread::sleep(std::time::Duration::from_secs(3));
    let running = ollama_running(config.ollama_port);
    info!("Ollama direct start: PID={}, running={}", pid, running);
    Ok((pid, running))
}
