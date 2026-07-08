use crate::config::Config;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub fn kilo_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("kilo")
        .join("kilo.json")
}

pub fn verify_kilo_config(config: &Config) -> anyhow::Result<KiloConfigStatus> {
    verify_kilo_config_from_path(&kilo_config_path(), config)
}

pub fn verify_kilo_config_from_path(
    path: &Path,
    _config: &Config,
) -> anyhow::Result<KiloConfigStatus> {
    if !path.exists() {
        return Ok(KiloConfigStatus {
            config_exists: false,
            indexing_configured: false,
            qdrant_configured: false,
            ollama_url: None,
            issues: vec!["kilo.json not found".into()],
        });
    }

    let content = std::fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&content)?;
    let mut issues = Vec::new();

    if json.get("indexing").is_some() {
        issues.push(
            "indexing property is not supported by the Kilo schema; run 'charm kilo init' to remove it"
                .into(),
        );
    }

    Ok(KiloConfigStatus {
        config_exists: true,
        indexing_configured: json.get("indexing").is_none(),
        qdrant_configured: json.get("indexing").is_none(),
        ollama_url: None,
        issues,
    })
}

pub struct KiloConfigStatus {
    pub config_exists: bool,
    pub indexing_configured: bool,
    pub qdrant_configured: bool,
    pub ollama_url: Option<String>,
    pub issues: Vec<String>,
}

pub fn generate_agents_md(config: &Config) -> String {
    let devops = config
        .devops_model
        .as_deref()
        .unwrap_or("qwen3-coder:30b-gpu");
    let quick = config
        .quick_model
        .as_deref()
        .unwrap_or("devstral-small-2-gpu");

    format!(
        r#"# charm-local-llm

Rust CLI that automates setup, optimization, and lifecycle management of local Ollama LLMs on CachyOS RTX 4090 and Apple Silicon MacBooks. Generates coding assistant configs for Crush and Kilocode so your entire AI toolchain runs locally.

## Project Structure

```text
charm-local-llm/
├── src/
│   ├── main.rs                  Entry point
│   ├── cli.rs                   clap CLI definitions
│   ├── commands.rs              start/stop/status/crush/kilo/etc
│   ├── config.rs                Config struct + platform defaults
│   ├── crush.rs                 Crush config (~/.config/crush/crush.json)
│   ├── kilo_integration.rs      Kilo config patching + AGENTS.md
│   ├── modelfile.rs             Ollama modelfile parser
│   ├── ollama.rs                Ollama HTTP API client
│   └── platform.rs              Platform detection + env loading
├── platform/
│   └── cachyos-i9-32gb-nvidia-4090/
│       ├── .env                 Platform env overrides
│       └── modfiles/            GPU-optimized model definitions
├── tests/integration_test.rs
├── docker-compose.yml           Qdrant vector DB
├── AGENTS.md / CRUSH.md         Auto-generated project context
└── Makefile
```

## Key Commands

- `charm start` — Start Ollama + models + Qdrant + generate Crush/Kilo configs
- `charm stop` — Stop everything
- `charm status` — Show environment status

## Local LLM Setup

- **Primary coding model**: `{devops}` (RTX 4090, 24GB VRAM)
- **Quick model**: `{quick}`
- **Embeddings**: `nomic-embed-text` (768 dims)
- **Ollama**: <http://localhost:{port}>
- **Qdrant**: <http://localhost:{qdrant}>

## Crush Integration

`charm start` generates `~/.config/crush/crush.json`:

- **Provider**: `ollama` at <http://localhost:{port}/v1/> with `discover_models: true`
- **large + medium** → `{devops}` (8192 max tokens)
- **small** → `{quick}` (4096 max tokens)
- **Context paths**: CRUSH.md, AGENTS.md, .clinerules

Also generates `CRUSH.md` in the project root as model context for Crush.

## Kilocode Integration

`charm start` generates `AGENTS.md` in the project root as context for Kilocode.

Kilo chat models route through the Kilo Gateway. Local Ollama is used only for chat model inference via the Gateway when local models are selected.

## Development

```bash
make build     # compile
make test      # run tests
make lint      # clippy + fmt + checkmake
make ci        # full CI pipeline
make start     # start Ollama environment
make setup     # install dependencies
```

## Code Style

- Follow Rust conventions
- Use `anyhow` for error handling
- No unnecessary comments
- Keep functions small and focused

## Integration Points

- Crush config: `~/.config/crush/crush.json` (auto-generated)
- AGENTS.md: project root context for Kilocode (auto-generated)
- CRUSH.md: project root context for Crush (auto-generated)
"#,
        devops = devops,
        quick = quick,
        port = config.ollama_port,
        qdrant = config.qdrant_port,
    )
}

pub fn write_agents_md(config: &Config, project_root: &Path) -> anyhow::Result<PathBuf> {
    let path = project_root.join("AGENTS.md");
    let content = generate_agents_md(config);
    std::fs::write(&path, content)?;
    info!("AGENTS.md written to {}", path.display());
    Ok(path)
}

pub fn patch_kilo_indexing(config: &Config) -> anyhow::Result<bool> {
    let path = kilo_config_path();
    patch_kilo_indexing_at_path(&path, config)
}

pub fn patch_kilo_indexing_at_path(path: &Path, config: &Config) -> anyhow::Result<bool> {
    if !path.exists() {
        warn!("kilo.json not found at {}, skipping patch", path.display());
        return Ok(false);
    }

    let content = std::fs::read_to_string(path)?;
    let mut json: Value = serde_json::from_str(&content)?;

    let obj = json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("kilo.json is not a JSON object"))?;

    let mut changed = false;

    if obj.remove("indexing").is_some() {
        info!("kilo.json cleaned up: removed unsupported indexing block");
        changed = true;
    }

    if patch_kilo_providers(obj, config)? {
        changed = true;
    }

    if changed {
        let new_content = serde_json::to_string_pretty(&json)?;
        std::fs::write(path, new_content)?;
        info!("kilo.json updated at {}", path.display());
    }

    Ok(changed)
}

fn patch_kilo_providers(
    obj: &mut serde_json::Map<String, Value>,
    config: &Config,
) -> anyhow::Result<bool> {
    let ollama_base = format!("http://localhost:{}/v1/", config.ollama_port);

    let ollama_provider = json!({
        "options": {
            "baseURL": ollama_base
        },
        "models": {
            "qwen3-coder:30b-gpu": { "name": "Qwen3 Coder 30B GPU" },
            "qwen3-coder:30b": { "name": "Qwen3 Coder 30B" },
            "qwen3:8b": { "name": "Qwen3 8B" },
            "gemma4:26b-devops": { "name": "Gemma 4 26B Devops" },
            "devstral-small-2-gpu": { "name": "Devstral Small 2 GPU" },
            "qwen2.5-coder:32b-devops": { "name": "Qwen 2.5 Coder 32B DevOps" },
            "qwen2.5-coder:14b-devops": { "name": "Qwen 2.5 Coder 14B DevOps" },
            "qwen2.5-coder:14b-quick": { "name": "Qwen 2.5 Coder 14B Quick" },
            "qwen2.5-coder:7b-quick": { "name": "Qwen 2.5 Coder 7B Quick" },
            "nomic-embed-text": { "name": "Nomic Embed Text" }
        }
    });

    let provider = obj.entry("provider").or_insert_with(|| json!({}));
    if let Some(provider_obj) = provider.as_object_mut() {
        let mut changed = false;

        if provider_obj.remove("ollama").is_some() {
            info!("Removed duplicate 'ollama' provider from kilo.json");
            changed = true;
        }

        let existing = provider_obj.get("Ollama Local (FREE)");
        if existing.is_none() || existing != Some(&ollama_provider) {
            provider_obj.insert("Ollama Local (FREE)".to_string(), ollama_provider);
            info!("Added Ollama Local (FREE) provider to kilo.json");
            changed = true;
        }

        return Ok(changed);
    }

    Ok(false)
}
