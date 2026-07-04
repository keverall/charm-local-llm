use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Modelfile {
    pub from: Option<String>,
    pub parameters: Vec<ModelParameter>,
    pub system: Option<String>,
    pub adapter: Option<String>,
    pub license: Option<String>,
    pub template: Option<String>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelParameter {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ModelfileOptimization {
    pub num_batch: Option<u32>,
    pub num_thread: Option<u32>,
    pub num_ctx: Option<u32>,
    pub num_gpu: Option<i32>,
    pub flash_attention: Option<bool>,
    pub kv_cache_type: Option<String>,
    pub recommended: Vec<String>,
}

pub fn parse_modelfile(path: &PathBuf) -> anyhow::Result<Modelfile> {
    let content = std::fs::read_to_string(path)?;
    parse_modelfile_content(&content)
}

pub fn parse_modelfile_content(content: &str) -> anyhow::Result<Modelfile> {
    let mut modelfile = Modelfile {
        from: None,
        parameters: vec![],
        system: None,
        adapter: None,
        license: None,
        template: None,
        messages: vec![],
    };

    let mut in_system = false;
    let mut system_content = String::new();
    let mut system_has_triple_quotes = false;

    for line in content.lines() {
        if line.starts_with("FROM ") || line.starts_with("FROM\t") {
            if let Some(val) = line.split_whitespace().nth(1) {
                modelfile.from = Some(val.trim().to_string());
            }
        } else if line.starts_with("#") {
            if !system_has_triple_quotes {
                in_system = false;
            }
        } else if line.trim_start().starts_with("SYSTEM ") || line.trim_start() == "SYSTEM" {
            in_system = true;
            system_content.clear();
            system_has_triple_quotes = false;
            if let Some(rest) = line.split_once("SYSTEM ").map(|x| x.1) {
                let rest = rest.trim_start();
                if rest.starts_with("\"\"\"") {
                    system_has_triple_quotes = true;
                    let content = rest.trim_start_matches("\"\"\"");
                    if content.ends_with("\"\"\"") {
                        system_content.push_str(content.trim_end_matches("\"\"\""));
                        in_system = false;
                        system_has_triple_quotes = false;
                    } else {
                        system_content.push_str(content);
                    }
                } else {
                    system_content.push_str(rest);
                }
            }
        } else if in_system {
            if system_has_triple_quotes {
                if line.ends_with("\"\"\"") {
                    let content = line.trim_end_matches("\"\"\"");
                    if !content.is_empty() || !system_content.is_empty() {
                        if !system_content.is_empty() {
                            system_content.push('\n');
                        }
                        system_content.push_str(content);
                    }
                    in_system = false;
                    system_has_triple_quotes = false;
                } else {
                    if !system_content.is_empty() {
                        system_content.push('\n');
                    }
                    system_content.push_str(line);
                }
            } else if line.is_empty() && !system_content.is_empty() {
                in_system = false;
            } else {
                if !system_content.is_empty() {
                    system_content.push('\n');
                }
                system_content.push_str(line);
            }
        } else if line.starts_with("TEMPLATE ") {
            if let Some(val) = line.split_whitespace().nth(1) {
                modelfile.template = Some(val.trim().to_string());
            }
        } else if line.starts_with("PARAMETER ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                modelfile.parameters.push(ModelParameter {
                    key: parts[1].to_string(),
                    value: parts[2].to_string(),
                });
            }
        } else if line.starts_with("ADAPTER ") {
            if let Some(val) = line.split_whitespace().nth(1) {
                modelfile.adapter = Some(val.trim().to_string());
            }
        } else if line.starts_with("LICENSE ") {
            if let Some(val) = line.split_whitespace().nth(1) {
                modelfile.license = Some(val.trim().to_string());
            }
        }
    }

    if !system_content.is_empty() {
        modelfile.system = Some(system_content.trim_end().to_string());
    }

    Ok(modelfile)
}

pub fn optimize_for_cachyos_4090(
    modelfile_path: &PathBuf,
) -> anyhow::Result<ModelfileOptimization> {
    let mf = parse_modelfile(modelfile_path)?;
    let mut opt = ModelfileOptimization {
        num_batch: None,
        num_thread: None,
        num_ctx: None,
        num_gpu: None,
        flash_attention: None,
        kv_cache_type: Some(String::from("q4_0")),
        recommended: vec![],
    };

    for p in &mf.parameters {
        match p.key.as_str() {
            "num_batch" => opt.num_batch = p.value.parse().ok(),
            "num_thread" => opt.num_thread = p.value.parse().ok(),
            "num_ctx" => opt.num_ctx = p.value.parse().ok(),
            "num_gpu" => opt.num_gpu = p.value.parse().ok(),
            _ => {}
        }
    }

    // RTX 4090 is 24GB VRAM, 14,000 CUDA cores
    // CachyOS with i9-13900KS has 24 cores / 32 threads
    if opt.num_batch.is_none() {
        opt.recommended
            .push(String::from("PARAMETER num_batch 512"));
        opt.num_batch = Some(512);
    }
    if opt.num_thread.is_none() {
        opt.recommended
            .push(String::from("PARAMETER num_thread 24"));
        opt.num_thread = Some(24);
    }
    if opt.num_ctx.is_none() {
        opt.recommended
            .push(String::from("PARAMETER num_ctx 32768"));
        opt.num_ctx = Some(32768);
    }
    if opt.num_gpu.is_none() {
        opt.recommended.push(String::from("PARAMETER num_gpu -1"));
        opt.num_gpu = Some(-1);
    }

    opt.flash_attention = Some(true);
    opt.recommended
        .push(String::from("OLLAMA_FLASH_ATTENTION=1"));
    opt.recommended
        .push(String::from("OLLAMA_KV_CACHE_TYPE=q4_0"));
    opt.recommended.push(String::from("OLLAMA_NUM_PARALLEL=24"));
    opt.recommended
        .push(String::from("OLLAMA_MAX_LOADED_MODELS=2"));
    opt.recommended.push(String::from("OLLAMA_GPU_LAYERS=50"));

    Ok(opt)
}

pub fn generate_optimized_modelfile(modelfile_path: &PathBuf) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(modelfile_path)?;
    let opt = optimize_for_cachyos_4090(modelfile_path)?;

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Update parameters
    let mut updated_lines: Vec<String> = vec![];
    let mut param_keys_seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in &lines {
        if line.starts_with("PARAMETER ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let key = parts[1];
                param_keys_seen.insert(key.to_string());
                match key {
                    "num_batch" => {
                        if let Some(v) = opt.num_batch {
                            updated_lines.push(format!("PARAMETER {} {}", key, v));
                        }
                    }
                    "num_thread" => {
                        if let Some(v) = opt.num_thread {
                            updated_lines.push(format!("PARAMETER {} {}", key, v));
                        }
                    }
                    "num_ctx" => {
                        if let Some(v) = opt.num_ctx {
                            updated_lines.push(format!("PARAMETER {} {}", key, v));
                        }
                    }
                    "num_gpu" => {
                        if let Some(v) = opt.num_gpu {
                            updated_lines.push(format!("PARAMETER {} {}", key, v));
                        }
                    }
                    _ => updated_lines.push(line.clone()),
                }
            } else {
                updated_lines.push(line.clone());
            }
        } else {
            updated_lines.push(line.clone());
        }
    }

    // Sync updated_lines back to lines
    lines = updated_lines;
    updated_lines = vec![];

    for line in &lines {
        if line.starts_with("PARAMETER ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && !param_keys_seen.contains(parts[1]) {
                match parts[1] {
                    "num_batch" if opt.num_batch.is_some() => {}
                    "num_thread" if opt.num_thread.is_some() => {}
                    "num_ctx" if opt.num_ctx.is_some() => {}
                    "num_gpu" if opt.num_gpu.is_some() => {}
                    _ => updated_lines.push(line.clone()),
                }
            } else {
                updated_lines.push(line.clone());
            }
        } else {
            updated_lines.push(line.clone());
        }
    }

    updated_lines.push(String::from(""));
    for rec in &opt.recommended {
        updated_lines.push(rec.clone());
    }

    Ok(updated_lines.join("\n"))
}
