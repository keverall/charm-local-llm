use crate::config::Config;
use crate::Platform;
use serde_json::{json, Value};
use std::collections::HashSet;
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
            "indexing property is not supported by the Kilo schema; run 'kcharm kilo init' to remove it"
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
        .unwrap_or("gemma4:26b-devops");
    let quick = config
        .quick_model
        .as_deref()
        .unwrap_or("devstral-small-2-gpu");
    let platform = config.platform;
    let platform_dir = platform.platform_dir();

    let memory_desc = if platform.is_macos() {
        match platform {
            Platform::MacOSM424Gb | Platform::MacOSM524Gb => {
                "Apple Silicon unified memory (24GB shared CPU/GPU)"
            }
            Platform::MacOSM432Gb | Platform::MacOSM532Gb => {
                "Apple Silicon unified memory (32GB shared CPU/GPU)"
            }
            _ => "Apple Silicon unified memory",
        }
    } else {
        "NVIDIA RTX 4090 (24GB VRAM)"
    };

    format!(
        r#"# charm-local-llm

Rust CLI that automates setup, optimization, and lifecycle management of local Ollama LLMs on CachyOS RTX 4090 and Apple Silicon MacBooks. Generates coding assistant configs for Crush and Kilocode so your entire AI toolchain runs locally.

## Project Structure

```text
charm-local-llm/
├── src/
│   ├── main.rs                  Entry point
│   ├── lib.rs                   Library crate (shared types / re-exports)
│   ├── cli.rs                   clap CLI definitions
│   ├── commands.rs              start/stop/status/crush/kilo/etc
│   ├── config.rs                Config struct + platform defaults
│   ├── crush.rs                 Crush config (~/.config/crush/crush.json)
│   ├── kilo_integration.rs      Kilo config patching + AGENTS.md
│   ├── modelfile.rs             Ollama modelfile parser
│   ├── ollama.rs                Ollama HTTP API client
│   └── platform.rs              Platform detection + env loading
├── platform/
│   ├── cachyos-i9-32gb-nvidia-4090/
│   ├── macos-m4-24gb/
│   ├── macos-m4-32gb/
│   ├── macos-m5-24gb/
│   └── macos-m5-32gb/           Platform env overrides + modfiles
├── tests/integration_test.rs
├── docker-compose.yml           Qdrant vector DB
├── AGENTS.md / CRUSH.md         Auto-generated project context
└── Makefile
```

## Key Commands

- `kcharm start` — Start Ollama + models + Qdrant + generate Crush/Kilo configs
- `kcharm stop` — Stop everything
- `kcharm status` — Show environment status

## Platform Detection

Auto-detected at runtime via `sysctl` (macOS) or `/etc/os-release` (Linux), or override with `--platform`:

| Platform | Directory | Memory/GPU | Primary Model | Quick Model |
|----------|-----------|------------|---------------|-------------|
| CachyOS RTX 4090 | `cachyos-i9-32gb-nvidia-4090` | 24GB VRAM | `gemma4:26b-devops` | `devstral-small-2-gpu` |
| macOS M4 24GB | `macos-m4-24gb` | 24GB unified | `qwen2.5-coder:14b-devops` | `qwen2.5-coder:7b-quick` |
| macOS M4 32GB | `macos-m4-32gb` | 32GB unified | `qwen3.6:27b-instruct-q4_K_M-devops` | `qwen2.5-coder:7b-quick` |
| macOS M5 24GB | `macos-m5-24gb` | 24GB unified | `qwen2.5-coder:14b-devops` | `qwen2.5-coder:7b-quick` |
| macOS M5 32GB | `macos-m5-32gb` | 32GB unified | `qwen3.6:27b-instruct-q4_K_M-devops` | `qwen2.5-coder:14b-quick` |

Override example: `kcharm start --platform macos-m5-32gb`

## Local LLM Setup

- **Current platform**: `{platform}` (`{platform_dir}`)
- **Memory/GPU**: {memory_desc}
- **Primary coding model**: `{devops}`
- **Quick model**: `{quick}`
- **Embeddings**: `nomic-embed-text` (768 dims)
- **Ollama**: <http://localhost:{port}>
- **Qdrant**: <http://localhost:{qdrant}>
- **Specialization**: Terraform, Ansible, YAML, JSON, TypeScript/JS/Node, Go, Python, Rust

## Crush Integration

`kcharm start` generates `~/.config/crush/crush.json`:

- **Provider**: `ollama` at <http://localhost:{port}/v1/> with `discover_models: true`
- **large + medium** → `{devops}` (8192 max tokens)
- **small** → `{quick}` (4096 max tokens)
- **Context paths**: CRUSH.md, AGENTS.md, .clinerules

Also generates `CRUSH.md` in the project root as model context for Crush.

## Kilocode Integration

`kcharm start` (and `kcharm kilo init`) writes `AGENTS.md` in the project root as context for Kilocode and patches `~/.config/kilo/kilo.json`:

- Registers an `Ollama Local (FREE)` provider pointing at the local Ollama endpoint (`http://localhost:{port}/v1/`) with known model aliases (including the platform devops/quick models).
- Removes any unsupported `indexing` block.

Kilocode then runs chat/inference directly against local Ollama — no external gateway, so data stays on-machine.

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
        platform = platform,
        platform_dir = platform_dir,
        memory_desc = memory_desc,
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
            "qwen3-coder:30b-gpu": { "name": "Qwen 3 Coder 30B GPU" },
            "gemma4:26b-devops": { "name": "Gemma 4 26B Devops" },
            "devstral-small-2-gpu": { "name": "Devstral Small 2 GPU" },
            "Qwen2.5-7B-instruct-GPU": { "name": "Qwen 2.5 7B Instruct GPU" },
            "qwen2.5-coder:14b-devops": { "name": "Qwen 2.5 Coder 14B DevOps" },
            "qwen2.5-coder:14b-quick": { "name": "Qwen 2.5 Coder 14B Quick" },
            "qwen2.5-coder:7b-quick": { "name": "Qwen 2.5 Coder 7B Quick" },
            "gemma4:e4b": { "name": "Gemma 4 E4B" },
            "qwen2.5-coder:3b": { "name": "Qwen 2.5 Coder 3B Quick" },
            "llama3.1:8b": { "name": "Llama 3.1 8B" },
            "nomic-embed-text:latest": { "name": "Nomic Embed Text" },
            "nomic-embed-text": { "name": "Nomic Embed Text" },
            "snowflake-arctic-embed": { "name": "Snowflake Arctic Embed" }
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

// ---------------------------------------------------------------------------
// .kiloignore composition
// ---------------------------------------------------------------------------
const BASE_KILOIGNORE: &str = include_str!("../assets/kilo/base.kiloignore");
const FRAGMENT_RUST: &str = include_str!("../assets/kilo/rust.kiloignore");
const FRAGMENT_GO: &str = include_str!("../assets/kilo/go.kiloignore");
const FRAGMENT_TS: &str = include_str!("../assets/kilo/ts.kiloignore");
const FRAGMENT_POWERSHELL: &str = include_str!("../assets/kilo/powershell.kiloignore");
const FRAGMENT_IAC: &str = include_str!("../assets/kilo/iac.kiloignore");
const FRAGMENT_PYTHON: &str = include_str!("../assets/kilo/python.kiloignore");

/// Which language/task fragments apply to `root`, based on marker files.
fn detect_kiloignore_fragments(root: &Path) -> Vec<&'static str> {
    let mut frags: Vec<&'static str> = Vec::new();
    let has = |name: &str| root.join(name).exists();

    if has("Cargo.toml") {
        frags.push(FRAGMENT_RUST);
    }
    if has("go.mod") {
        frags.push(FRAGMENT_GO);
    }
    if has("package.json") {
        frags.push(FRAGMENT_TS);
    }
    if has("pyproject.toml")
        || has("requirements.txt")
        || has("setup.py")
        || has("Pipfile")
        || has("poetry.lock")
    {
        frags.push(FRAGMENT_PYTHON);
    }
    if has("main.tf")
        || has("terraform.tf")
        || has("variables.tf")
        || has("ansible.cfg")
        || has("playbook.yml")
        || has("playbook.yaml")
        || has("Chart.yaml")
    {
        frags.push(FRAGMENT_IAC);
    }
    let ps = std::fs::read_dir(root)
        .ok()
        .map(|d| {
            d.filter_map(|e| e.ok())
                .any(|e| {
                    let n = e.file_name().to_string_lossy().to_lowercase();
                    n.ends_with(".ps1") || n.ends_with(".psm1") || n.ends_with(".psd1")
                })
        })
        .unwrap_or(false);
    if ps {
        frags.push(FRAGMENT_POWERSHELL);
    }
    frags
}

/// Append non-empty, non-comment, de-duplicated lines from `src` into `lines`.
fn append_kiloignore_lines(
    lines: &mut Vec<String>,
    seen: &mut HashSet<String>,
    src: &str,
) {
    for raw in src.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            lines.push(line.to_string());
            continue;
        }
        if seen.insert(line.to_string()) {
            lines.push(line.to_string());
        }
    }
}

/// Compose the final `.kiloignore` content: base + applicable fragments.
pub fn compose_kiloignore(project_root: &Path) -> String {
    let mut seen = HashSet::new();
    let mut lines: Vec<String> = Vec::new();
    append_kiloignore_lines(&mut lines, &mut seen, BASE_KILOIGNORE);
    for frag in detect_kiloignore_fragments(project_root) {
        append_kiloignore_lines(&mut lines, &mut seen, frag);
    }
    lines.join("\n") + "\n"
}

/// Ensure a `.kiloignore` exists in the project root, composing base rules with
/// language/task fragments. Non-destructive: if one already exists it is left
/// untouched and `false` is returned.
pub fn ensure_kiloignore(config: &Config) -> anyhow::Result<bool> {
    let dest = config.project_root.join(".kiloignore");
    if dest.exists() {
        info!(
            ".kiloignore already exists at {}; leaving untouched",
            dest.display()
        );
        return Ok(false);
    }
    let content = compose_kiloignore(&config.project_root);
    std::fs::write(&dest, content)?;
    info!(".kiloignore written to {}", dest.display());
    Ok(true)
}
