use crate::config::Config;
use serde_json::Value;
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
    config: &Config,
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
    let mut indexing_ok = false;
    let mut qdrant_ok = false;
    let mut ollama_url = None;

    if let Some(indexing) = json.get("indexing") {
        let provider = indexing
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let base_url = indexing
            .get("ollama")
            .and_then(|o| o.get("baseUrl"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if provider == "ollama" {
            if base_url.starts_with("http://localhost") {
                indexing_ok = true;
                ollama_url = Some(base_url.to_string());
            } else {
                issues.push(format!(
                    "indexing ollama baseUrl '{}' should be local",
                    base_url
                ));
            }
        } else if provider.is_empty() {
            issues.push("indexing provider not set (should be 'ollama')".into());
        } else {
            issues.push(format!(
                "indexing provider is '{}' (should be 'ollama')",
                provider
            ));
        }

        let qdrant_url = indexing
            .get("qdrant")
            .and_then(|q| q.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let expected_qdrant = format!("http://localhost:{}", config.qdrant_port);
        if qdrant_url == expected_qdrant {
            qdrant_ok = true;
        } else if qdrant_url.is_empty() {
            issues.push("qdrant url not set in indexing config".into());
        } else {
            issues.push(format!(
                "qdrant url '{}' should be '{}'",
                qdrant_url, expected_qdrant
            ));
        }
    } else {
        issues.push("indexing section missing from kilo.json".into());
    }

    Ok(KiloConfigStatus {
        config_exists: true,
        indexing_configured: indexing_ok,
        qdrant_configured: qdrant_ok,
        ollama_url,
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

Rust CLI that automates setup, optimization, and lifecycle management of local Ollama LLMs on CachyOS with NVIDIA RTX 4090. Generates coding assistant configs for Crush and Kilocode so your entire AI toolchain runs locally.

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
- **Ollama**: http://localhost:{port}
- **Qdrant**: http://localhost:{qdrant}

## Crush Integration
`charm start` generates `~/.config/crush/crush.json`:
- **Provider**: `ollama` at `http://localhost:{port}/v1/` with `discover_models: true`
- **large + medium** → `{devops}` (8192 max tokens)
- **small** → `{quick}` (4096 max tokens)
- **Context paths**: CRUSH.md, AGENTS.md, .clinerules

Also generates `CRUSH.md` in the project root as model context for Crush.

## Kilocode Integration
`charm start` patches `~/.config/kilo/kilo.json` indexing section:
- **Provider**: `ollama`, **baseUrl**: `http://localhost:{port}`
- **Model**: `nomic-embed-text` (768 dims)
- **Vector store**: `qdrant` at `http://localhost:{qdrant}`

Kilo chat models route through the Kilo Gateway (not Ollama). Local Ollama is used only for code indexing and semantic search.

Also generates `AGENTS.md` in the project root as context for Kilocode.

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
- Kilo indexing: `~/.config/kilo/kilo.json` (auto-patched)
- CRUSH.md: project root context (auto-generated)
- AGENTS.md: project root context (auto-generated)
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
    let mut changed = false;

    let obj = json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("kilo.json is not a JSON object"))?;

    let indexing = obj
        .entry("indexing")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    let expected_url = format!("http://localhost:{}", config.ollama_port);
    let expected_qdrant = format!("http://localhost:{}", config.qdrant_port);

    if let Some(idx) = indexing.as_object_mut() {
        if idx.get("provider").and_then(|v| v.as_str()) != Some("ollama") {
            idx.insert("provider".into(), Value::String("ollama".into()));
            changed = true;
        }

        let ollama_entry = idx
            .entry("ollama")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(ollama) = ollama_entry.as_object_mut() {
            if ollama.get("baseUrl").and_then(|v| v.as_str()) != Some(&expected_url) {
                ollama.insert("baseUrl".into(), Value::String(expected_url));
                changed = true;
            }
        }

        if idx.get("model").and_then(|v| v.as_str()) != Some("nomic-embed-text") {
            idx.insert("model".into(), Value::String("nomic-embed-text".into()));
            changed = true;
        }
        if idx.get("dimension").and_then(|v| v.as_u64()) != Some(768) {
            idx.insert(
                "dimension".into(),
                Value::Number(serde_json::Number::from(768)),
            );
            changed = true;
        }

        let qdrant_entry = idx
            .entry("qdrant")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(qdrant) = qdrant_entry.as_object_mut() {
            if qdrant.get("url").and_then(|v| v.as_str()) != Some(&expected_qdrant) {
                qdrant.insert("url".into(), Value::String(expected_qdrant));
                changed = true;
            }
        }

        if idx.get("vectorStore").and_then(|v| v.as_str()) != Some("qdrant") {
            idx.insert("vectorStore".into(), Value::String("qdrant".into()));
            changed = true;
        }
        if idx.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
            idx.insert("enabled".into(), Value::Bool(true));
            changed = true;
        }
    }

    if changed {
        let new_content = serde_json::to_string_pretty(&json)?;
        std::fs::write(path, new_content)?;
        info!("kilo.json patched with local Ollama indexing config");
    }

    Ok(changed)
}
