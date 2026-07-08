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
    MacOSM424Gb,
    MacOSM432Gb,
    MacOSM524Gb,
    MacOSM532Gb,
    CachyOS,
    Linux,
    Unknown,
}

impl Platform {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().replace('_', "-").as_str() {
            "macos" | "macbook" | "macos-m4-24gb" | "macbook-m4-24gb" | "m4-24gb" => {
                Platform::MacOSM424Gb
            }
            "macos-m4-32gb" | "macbook-m4-32gb" | "m4-32gb" => Platform::MacOSM432Gb,
            "macos-m5-24gb" | "macbook-m5-24gb" | "m5-24gb" => Platform::MacOSM524Gb,
            "macos-m5-32gb" | "macbook-m5-32gb" | "m5-32gb" => Platform::MacOSM532Gb,
            "cachyos" => Platform::CachyOS,
            "linux" => Platform::Linux,
            _ => Platform::Unknown,
        }
    }

    pub fn is_macos(self) -> bool {
        matches!(
            self,
            Platform::MacOS
                | Platform::MacOSM424Gb
                | Platform::MacOSM432Gb
                | Platform::MacOSM524Gb
                | Platform::MacOSM532Gb
        )
    }

    pub fn platform_dir(self) -> &'static str {
        match self {
            Platform::MacOS | Platform::MacOSM424Gb => "macos-m4-24gb",
            Platform::MacOSM432Gb => "macos-m4-32gb",
            Platform::MacOSM524Gb => "macos-m5-24gb",
            Platform::MacOSM532Gb => "macos-m5-32gb",
            Platform::CachyOS | Platform::Linux | Platform::Unknown => {
                "cachyos-i9-32gb-nvidia-4090"
            }
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::MacOS => write!(f, "macos"),
            Platform::MacOSM424Gb => write!(f, "macos-m4-24gb"),
            Platform::MacOSM432Gb => write!(f, "macos-m4-32gb"),
            Platform::MacOSM524Gb => write!(f, "macos-m5-24gb"),
            Platform::MacOSM532Gb => write!(f, "macos-m5-32gb"),
            Platform::CachyOS => write!(f, "cachyos"),
            Platform::Linux => write!(f, "linux"),
            Platform::Unknown => write!(f, "unknown"),
        }
    }
}

impl Config {
    pub fn new(platform: Platform, project_root: &Path) -> Self {
        let qdrant_data_dir = match platform {
            p if p.is_macos() => project_root.join("data").join("qdrant"),
            Platform::CachyOS | Platform::Linux => dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".qdrant"),
            Platform::Unknown => project_root.join("data").join("qdrant"),
            _ => project_root.join("data").join("qdrant"),
        };

        let (
            default_models,
            devops_model,
            quick_model,
            ollama_kv_cache_type,
            ollama_flash_attention,
            ollama_gpu_layers,
        ) = match platform {
            Platform::MacOS | Platform::MacOSM424Gb | Platform::MacOSM524Gb => (
                vec![
                    "qwen2.5-coder:14b-devops".into(),
                    "nomic-embed-text:latest".into(),
                ],
                Some("qwen2.5-coder:14b-devops".into()),
                Some("qwen2.5-coder:7b-quick".into()),
                "q4_0".into(),
                Some(1),
                None,
            ),
            Platform::MacOSM432Gb => (
                vec![
                    "qwen3.6:27b-instruct-q4_K_M-devops".into(),
                    "nomic-embed-text:latest".into(),
                ],
                Some("qwen3.6:27b-instruct-q4_K_M-devops".into()),
                Some("qwen2.5-coder:7b-quick".into()),
                "q4_0".into(),
                Some(1),
                None,
            ),
            Platform::MacOSM532Gb => (
                vec![
                    "qwen3.6:27b-instruct-q4_K_M-devops".into(),
                    "nomic-embed-text:latest".into(),
                ],
                Some("qwen3.6:27b-instruct-q4_K_M-devops".into()),
                Some("qwen2.5-coder:14b-quick".into()),
                "q4_0".into(),
                Some(1),
                None,
            ),
            Platform::CachyOS | Platform::Linux => (
                vec![
                    "qwen3.6:27b-instruct-q4_K_M-gpu".into(),
                    "qwen3:8b".into(),
                    "nomic-embed-text:latest".into(),
                ],
                Some("qwen3.6:27b-instruct-q4_K_M-gpu".into()),
                Some("devstral-small-2-gpu".into()),
                "q4_0".into(),
                None,
                Some(50),
            ),
            Platform::Unknown => (
                vec!["qwen3.6:27b-instruct-q4_K_M-gpu".into()],
                Some("gemma4:26b-devops".into()),
                Some("devstral-small-2-gpu".into()),
                "q4_0".into(),
                None,
                Some(50),
            ),
        };

        Self {
            platform,
            ollama_host: if platform.is_macos() {
                "127.0.0.1:11434".into()
            } else {
                "[::]:11434".into()
            },
            ollama_port: 11434,
            ollama_bin: "ollama".into(),
            ollama_base_url: "http://localhost:11434".into(),
            ollama_num_parallel: if platform.is_macos() { 4 } else { 24 },
            ollama_max_loaded_models: if platform.is_macos() { 1 } else { 2 },
            ollama_kv_cache_type,
            ollama_flash_attention,
            ollama_gpu_layers,
            ollama_models_path: Some(if platform.is_macos() {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".ollama")
                    .join("models")
            } else {
                PathBuf::from("/home/ollama/models")
            }),
            default_models,
            devops_model,
            quick_model,
            qdrant_port: 6333,
            qdrant_grpc_port: 6334,
            qdrant_data_dir,
            modfile_dir: project_root
                .join("platform")
                .join(platform.platform_dir())
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
            let value = if let Some(stripped) = v.strip_prefix("~/") {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(stripped)
            } else {
                PathBuf::from(v)
            };
            self.ollama_models_path = Some(value);
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
        self.update_paths_from_project_root();
        self
    }

    pub fn update_paths_from_project_root(&mut self) {
        self.modfile_dir = self
            .project_root
            .join("platform")
            .join(self.platform.platform_dir())
            .join("modfiles");
    }

    /// CachyOS runs on a single RTX 4090 (24GB VRAM). These are the only
    /// memory parameters that keep concurrent model loads within the single-GPU
    /// budget. Anything that would violate the single-GPU contract is rejected
    /// rather than silently printed, so operators never boot an OOM-prone config.
    pub fn validate_cachyos_single_gpu_profile(&self) -> anyhow::Result<()> {
        const EXPECTED_GPU_LAYERS: u16 = 50;
        const EXPECTED_NUM_PARALLEL: u16 = 24;
        const EXPECTED_MAX_LOADED: u16 = 2;

        if self.ollama_num_parallel == 0 {
            anyhow::bail!(
                "OLLAMA_NUM_PARALLEL must be >= 1 for the single-GPU CachyOS profile (got 0)"
            );
        }
        if self.ollama_max_loaded_models == 0 {
            anyhow::bail!(
                "OLLAMA_MAX_LOADED_MODELS must be >= 1 for the single-GPU CachyOS profile (got 0)"
            );
        }
        match self.ollama_gpu_layers {
            None => anyhow::bail!(
                "OLLAMA_GPU_LAYERS must be set for the single-GPU CachyOS profile (got unset)"
            ),
            Some(0) => anyhow::bail!(
                "OLLAMA_GPU_LAYERS must be > 0 for the single-GPU CachyOS profile (got 0)"
            ),
            _ => {}
        }

        if self.ollama_gpu_layers != Some(EXPECTED_GPU_LAYERS)
            || self.ollama_num_parallel != EXPECTED_NUM_PARALLEL
            || self.ollama_max_loaded_models != EXPECTED_MAX_LOADED
        {
            anyhow::bail!(
                "CachyOS single-GPU memory profile overridden: gpu_layers={:?} (expected {}), \
                 num_parallel={} (expected {}), max_loaded_models={} (expected {}); \
                 these must stay within the 24GB RTX 4090 budget",
                self.ollama_gpu_layers,
                EXPECTED_GPU_LAYERS,
                self.ollama_num_parallel,
                EXPECTED_NUM_PARALLEL,
                self.ollama_max_loaded_models,
                EXPECTED_MAX_LOADED
            );
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("env file not found: {0}")]
    EnvFileNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
