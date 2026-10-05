use crate::config::Config;
use crate::modelfile::parse_modelfile_content;
use crate::platform::find_ollama_bin;
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
    download_client: Client,
    config: Config,
}

impl OllamaClient {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        // No timeout — used for model pull/create which can take many minutes.
        let download_client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap_or_default();

        Self {
            client,
            download_client,
            config,
        }
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
                    size: m
                        .get("size")
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
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

    pub async fn model_modelfile(&self, model_name: &str) -> anyhow::Result<Option<String>> {
        let url = format!("{}/api/show", self.base_url());
        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "name": model_name }))
            .send()
            .await?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let body: serde_json::Value = response.json().await?;
        Ok(body
            .get("modelfile")
            .and_then(|value| value.as_str())
            .map(str::to_owned))
    }

    pub async fn pull_model(&self, model_name: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/pull", self.base_url());
        let payload = serde_json::json!({ "name": model_name, "stream": false });
        info!("Pulling model: {}", model_name);
        let resp = self
            .download_client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to pull model: {} - {}",
                status,
                text
            ));
        }

        resp.json::<serde_json::Value>().await?;
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
            "stream": false,
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

        // With stream:false Ollama returns a single JSON response once the
        // model is fully written to disk. We read it to completion so the
        // server finishes before we declare success.
        let resp = self
            .download_client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to create model from modelfile: {} - {}",
                status,
                text
            ));
        }

        // With stream:false Ollama returns a single JSON response once the
        // model is fully written to disk. We read it to completion so the
        // server finishes before we declare success.
        resp.json::<serde_json::Value>().await?;

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
    ) -> anyhow::Result<bool> {
        let existing = self.list_models().await?;
        let existing_name = existing
            .iter()
            .find(|m| {
                m.name.strip_suffix(":latest").unwrap_or(&m.name)
                    == model_name.strip_suffix(":latest").unwrap_or(model_name)
            })
            .map(|m| m.name.as_str());
        let already_present = existing_name.is_some();

        if already_present {
            let Some(mf_name) = modelfile_name else {
                info!("Model '{}' already present; no change needed.", model_name);
                return Ok(false);
            };
            let mf_path = self.config.modfile_dir.join(mf_name);
            if !mf_path.exists() {
                info!(
                    "Model '{}' already present; modelfile unavailable, preserving it.",
                    model_name
                );
                return Ok(false);
            }
            let modelfile_modified = std::fs::metadata(&mf_path)?.modified()?;
            let model_modified: Option<std::time::SystemTime> = existing
                .iter()
                .find(|m| Some(m.name.as_str()) == existing_name)
                .and_then(|m| chrono::DateTime::parse_from_rfc3339(&m.modified_at).ok())
                .map(|timestamp| timestamp.into());
            if model_modified.is_some_and(|timestamp| timestamp >= modelfile_modified) {
                info!(
                    "Model '{}' is newer than its modelfile; no change needed.",
                    model_name
                );
                return Ok(false);
            }
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
                        "Failed to create model '{}' from modfile: {}",
                        model_name, e
                    );
                    return Err(e);
                }
                return Ok(true);
            }
            warn!("Modfile not found at {:?}", mf_path);
        }

        self.pull_model(model_name).await.map(|()| true)
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
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    // Probe every common loopback address. Ollama may bind IPv6 ([::]) while a
    // health check only hits 127.0.0.1 (IPv4) — without this, a perfectly
    // healthy server reports as "not running" and `kcharm start` bails out
    // before it ever patches the Kilo/Crush configs.
    for host in ["127.0.0.1", "[::1]", "localhost"] {
        if let Ok(resp) = client
            .get(format!("http://{}:{}/api/tags", host, port))
            .send()
        {
            if resp.status().is_success() {
                return true;
            }
        }
    }
    false
}

fn ollama_executable(config: &Config) -> PathBuf {
    if config.ollama_bin == "ollama" {
        find_ollama_bin().unwrap_or_else(|| PathBuf::from("ollama"))
    } else {
        PathBuf::from(&config.ollama_bin)
    }
}

/// Return the version of the running Ollama server, if its version endpoint is
/// available. This is deliberately separate from `ollama_running`: a healthy
/// server may still be running an older binary after pacman upgraded Ollama.
pub fn ollama_server_version(port: u16) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let response = client
        .get(format!("http://127.0.0.1:{}/api/version", port))
        .send()
        .ok()?
        .error_for_status()
        .ok()?;
    response
        .json::<serde_json::Value>()
        .ok()?
        .get("version")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Return the installed Ollama client version. Ollama prints the server
/// version first when connected, followed by `client version is ...` when the
/// two differ, so prefer the explicit client-version line when present.
pub fn ollama_binary_version(config: &Config) -> Option<String> {
    let output = Command::new(ollama_executable(config))
        .arg("--version")
        .output()
        .ok()?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let client_marker = "client version is ";
    text.lines()
        .find_map(|line| line.split_once(client_marker).map(|(_, v)| v.trim()))
        .or_else(|| {
            text.lines()
                .find_map(|line| line.strip_prefix("ollama version is ").map(str::trim))
        })
        .map(str::to_owned)
}

pub fn ollama_version_mismatch(config: &Config) -> Option<(String, String)> {
    let server = ollama_server_version(config.ollama_port)?;
    let client = ollama_binary_version(config)?;
    (server != client).then_some((client, server))
}

pub fn start_ollama_direct(config: &Config, log_file: &PathBuf) -> anyhow::Result<(u32, bool)> {
    let executable = ollama_executable(config);
    let mut cmd = Command::new(executable);
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
        if let Some(ka) = &config.ollama_keep_alive {
            cmd.env("OLLAMA_KEEP_ALIVE", ka);
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
