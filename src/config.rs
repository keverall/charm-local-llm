use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub platform: Platform,
    pub ollama_host: String,
    pub ollama_port: u16,
    pub ollama_bin: String,
    pub ollama_base_url: String,
    pub ollama_num_parallel: u16,
    pub ollama_max_loaded_models: u16,
    pub ollama_kv_cache_type: String,
    pub ollama_flash_attention: Option<u8>,
    pub ollama_gpu_layers: Option<u16>,
    pub ollama_models_path: Option<PathBuf>,
    pub default_models: Vec<String>,
    pub devops_model: Option<String>,
    pub quick_model: Option<String>,
    pub qdrant_port: u16,
    pub qdrant_grpc_port: u16,
    pub qdrant_data_dir: PathBuf,
    pub modfile_dir: PathBuf,
    pub project_root: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    CachyOS,
    Linux,
    Unknown,
}

impl Platform {
    pub fn from_string(s: &str) -> Self {
        match s {
            "macos" | "macbook" => Platform::MacOS,
            "cachyos" => Platform::CachyOS,
            "linux" => Platform::Linux,
            _ => Platform::Unknown,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::MacOS => write!(f, "macos"),
            Platform::CachyOS => write!(f, "cachyos"),
            Platform::Linux => write!(f, "linux"),
            Platform::Unknown => write!(f, "unknown"),
        }
    }
}

impl Config {
    pub fn new(platform: Platform, project_root: &Path) -> Self {
        let qdrant_data_dir = match platform {
            Platform::MacOS => project_root.join("data").join("qdrant"),
            Platform::CachyOS | Platform::Linux => dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".qdrant"),
            Platform::Unknown => project_root.join("data").join("qdrant"),
        };

        let (
            default_models,
            devops_model,
            quick_model,
            ollama_kv_cache_type,
            ollama_flash_attention,
            ollama_gpu_layers,
        ) = match platform {
            Platform::MacOS => (
                vec!["qwen-devops".into(), "nomic-embed-text:latest".into()],
                Some("qwen-devops".into()),
                None,
                "q4_0".into(),
                Some(1),
                None,
            ),
            Platform::CachyOS | Platform::Linux => (
                vec![
                    "qwen3-coder:30b-gpu".into(),
                    "nomic-embed-text:latest".into(),
                ],
                Some("qwen3-coder:30b-gpu".into()),
                Some("devstral-small-2-gpu".into()),
                "q4_0".into(),
                None,
                Some(50),
            ),
            Platform::Unknown => (
                vec!["qwen3-coder:30b-gpu".into()],
                Some("gemma4:26b-devops".into()),
                Some("devstral-small-2-gpu".into()),
                "q4_0".into(),
                None,
                Some(50),
            ),
        };

        Self {
            platform,
            ollama_host: "[::]:11434".into(),
            ollama_port: 11434,
            ollama_bin: "ollama".into(),
            ollama_base_url: "http://localhost:11434".into(),
            ollama_num_parallel: 24,
            ollama_max_loaded_models: 2,
            ollama_kv_cache_type,
            ollama_flash_attention,
            ollama_gpu_layers,
            ollama_models_path: Some(PathBuf::from("/home/ollama/models")),
            default_models,
            devops_model,
            quick_model,
            qdrant_port: 6333,
            qdrant_grpc_port: 6334,
            qdrant_data_dir,
            modfile_dir: project_root
                .join("platform")
                .join("cachyos-i9-32gb-nvidia-4090")
                .join("modfiles"),
            project_root: project_root.to_path_buf(),
        }
    }

    pub fn default(platform: Platform) -> Self {
        let project_root = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("repos")
            .join("ollama");
        Self::new(platform, &project_root)
    }

    pub fn with_env_overrides(mut self, env: std::collections::HashMap<String, String>) -> Self {
        if let Some(v) = env.get("OLLAMA_HOST") {
            self.ollama_host = v.clone();
        }
        if let Some(v) = env.get("OLLAMA_PORT") {
            if let Ok(p) = v.parse::<u16>() {
                self.ollama_port = p;
                self.ollama_base_url = format!("http://localhost:{}", p);
            }
        }
        if let Some(v) = env.get("OLLAMA_BIN") {
            self.ollama_bin = v.clone();
        }
        if let Some(v) = env.get("OLLAMA_NUM_PARALLEL") {
            if let Ok(p) = v.parse::<u16>() {
                self.ollama_num_parallel = p;
            }
        }
        if let Some(v) = env.get("OLLAMA_MAX_LOADED_MODELS") {
            if let Ok(p) = v.parse::<u16>() {
                self.ollama_max_loaded_models = p;
            }
        }
        if let Some(v) = env.get("OLLAMA_KV_CACHE_TYPE") {
            self.ollama_kv_cache_type = v.clone();
        }
        if let Some(v) = env.get("OLLAMA_FLASH_ATTENTION") {
            if let Ok(f) = v.parse::<u8>() {
                self.ollama_flash_attention = Some(f);
            }
        }
        if let Some(v) = env.get("OLLAMA_GPU_LAYERS") {
            if let Ok(l) = v.parse::<u16>() {
                self.ollama_gpu_layers = Some(l);
            }
        }
        if let Some(v) = env.get("OLLAMA_MODELS") {
            self.ollama_models_path = Some(PathBuf::from(v));
        }
        if let Some(v) = env.get("DEFAULT_MODELS") {
            self.default_models = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Some(v) = env.get("DEVOPS_MODEL") {
            self.devops_model = Some(v.clone());
        }
        if let Some(v) = env.get("QUICK_MODEL") {
            self.quick_model = Some(v.clone());
        }
        if let Some(v) = env.get("QDRANT_PORT") {
            if let Ok(p) = v.parse::<u16>() {
                self.qdrant_port = p;
            }
        }
        if let Some(v) = env.get("QDRANT_GRPC_PORT") {
            if let Ok(p) = v.parse::<u16>() {
                self.qdrant_grpc_port = p;
            }
        }
        if let Some(v) = env.get("QDRANT_DATA_DIR") {
            self.qdrant_data_dir = PathBuf::from(v);
        }
        if let Some(v) = env.get("PLATFORM_OVERRIDE") {
            self.platform = Platform::from_string(v);
        }
        self
    }

    pub fn update_paths_from_project_root(&mut self) {
        self.modfile_dir = self
            .project_root
            .join("platform")
            .join(match self.platform {
                Platform::MacOS => "macbook-m4-24gb-optimized",
                Platform::CachyOS | Platform::Linux | Platform::Unknown => {
                    "cachyos-i9-32gb-nvidia-4090"
                }
            })
            .join("modfiles");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("env file not found: {0}")]
    EnvFileNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
