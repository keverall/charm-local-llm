use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct CrushConfig {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub providers: BTreeMap<String, CrushProvider>,
    pub models: BTreeMap<String, CrushSelectedModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<CrushOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<CrushPermissions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrushProvider {
    pub name: String,
    pub base_url: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub discover_models: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<CrushModel>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrushModel {
    pub id: String,
    pub name: String,
    pub context_window: u32,
    pub default_max_tokens: u32,
    pub cost_per_1m_in: f64,
    pub cost_per_1m_out: f64,
    pub cost_per_1m_in_cached: f64,
    pub cost_per_1m_out_cached: f64,
    pub can_reason: bool,
    pub supports_attachments: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrushSelectedModel {
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrushOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_context_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<CrushAttribution>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrushAttribution {
    pub trailer_style: String,
    pub generated_with: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrushPermissions {
    pub allowed_tools: Vec<String>,
}

pub fn crush_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("crush")
        .join("crush.json")
}

pub fn build_crush_config(config: &Config) -> CrushConfig {
    let ollama_base = format!("http://localhost:{}/v1/", config.ollama_port);
    let devops_model = config
        .devops_model
        .as_deref()
        .unwrap_or("qwen3-coder:30b-gpu");
    let quick_model = config
        .quick_model
        .as_deref()
        .unwrap_or("devstral-small-2-gpu");
    let embed_model = "nomic-embed-text";

    let known_models = vec![
        ("qwen3-coder:30b-gpu", "Qwen3 Coder 30B GPU"),
        ("qwen3-coder:30b", "Qwen3 Coder 30B"),
        ("qwen3:8b", "Qwen3 8B"),
        ("gemma4:26b-devops", "Gemma 4 26B Devops"),
        ("gemma4:26b", "Gemma 4 26B"),
        ("devstral-small-2-gpu", "Devstral Small 2 GPU"),
        ("devstral-small-2", "Devstral Small 2"),
        ("qwen2.5-coder:32b-devops", "Qwen 2.5 Coder 32B DevOps"),
        ("qwen2.5-coder:14b-devops", "Qwen 2.5 Coder 14B DevOps"),
        ("qwen2.5-coder:14b-quick", "Qwen 2.5 Coder 14B Quick"),
        ("qwen2.5-coder:7b-quick", "Qwen 2.5 Coder 7B Quick"),
        ("qwen2.5-coder:32b", "Qwen 2.5 Coder 32B"),
        ("qwen2.5-coder:14b", "Qwen 2.5 Coder 14B"),
        ("qwen2.5-coder:7b", "Qwen 2.5 Coder 7B"),
        ("nomic-embed-text", "Nomic Embed Text"),
        ("nomic-embed-text:latest", "Nomic Embed Text"),
    ];

    let label = |id: &str| {
        known_models
            .iter()
            .find(|(k, _)| *k == id)
            .map(|(_, name)| (*name).to_string())
            .unwrap_or_else(|| id.to_string())
    };

    let mut additional_models = vec![];
    for known in &known_models {
        let id = known.0;
        if id != devops_model && id != quick_model && id != embed_model {
            additional_models.push(CrushModel {
                id: id.into(),
                name: known.1.into(),
                context_window: 32768,
                default_max_tokens: 8192,
                cost_per_1m_in: 0.0,
                cost_per_1m_out: 0.0,
                cost_per_1m_in_cached: 0.0,
                cost_per_1m_out_cached: 0.0,
                can_reason: id.contains("qwen") || id.contains("gemma") || id.contains("devstral"),
                supports_attachments: !id.contains("embed"),
            });
        }
    }

    let mut models_list = vec![
        CrushModel {
            id: devops_model.into(),
            name: label(devops_model),
            context_window: 32768,
            default_max_tokens: 8192,
            cost_per_1m_in: 0.0,
            cost_per_1m_out: 0.0,
            cost_per_1m_in_cached: 0.0,
            cost_per_1m_out_cached: 0.0,
            can_reason: true,
            supports_attachments: true,
        },
        CrushModel {
            id: quick_model.into(),
            name: label(quick_model),
            context_window: 32768,
            default_max_tokens: 8192,
            cost_per_1m_in: 0.0,
            cost_per_1m_out: 0.0,
            cost_per_1m_in_cached: 0.0,
            cost_per_1m_out_cached: 0.0,
            can_reason: false,
            supports_attachments: true,
        },
        CrushModel {
            id: embed_model.into(),
            name: label(embed_model),
            context_window: 8192,
            default_max_tokens: 8192,
            cost_per_1m_in: 0.0,
            cost_per_1m_out: 0.0,
            cost_per_1m_in_cached: 0.0,
            cost_per_1m_out_cached: 0.0,
            can_reason: false,
            supports_attachments: false,
        },
    ];
    models_list.extend(additional_models);

    let ollama_provider = CrushProvider {
        name: "Ollama Local".into(),
        base_url: ollama_base,
        provider_type: "ollama".into(),
        api_key: Some("ollama".into()),
        discover_models: true,
        models: Some(models_list),
    };

    let mut providers = BTreeMap::new();
    providers.insert("ollama".into(), ollama_provider);

    let mut models = BTreeMap::new();
    models.insert(
        "large".into(),
        CrushSelectedModel {
            model: devops_model.into(),
            provider: "ollama".into(),
            max_tokens: Some(8192),
        },
    );
    models.insert(
        "medium".into(),
        CrushSelectedModel {
            model: devops_model.into(),
            provider: "ollama".into(),
            max_tokens: Some(8192),
        },
    );
    models.insert(
        "small".into(),
        CrushSelectedModel {
            model: quick_model.into(),
            provider: "ollama".into(),
            max_tokens: Some(4096),
        },
    );

    let options = CrushOptions {
        context_paths: Some(vec![
            "CRUSH.md".into(),
            "AGENTS.md".into(),
            ".clinerules".into(),
        ]),
        global_context_paths: Some(vec![
            "~/.config/crush/CRUSH.md".into(),
            "~/.config/AGENTS.md".into(),
        ]),
        skills_paths: Some(vec!["~/.config/crush/skills".into(), "./skills".into()]),
        attribution: Some(CrushAttribution {
            trailer_style: "co-authored-by".into(),
            generated_with: true,
        }),
    };

    let permissions = CrushPermissions {
        allowed_tools: vec![
            "bash".into(),
            "view".into(),
            "edit".into(),
            "write".into(),
            "glob".into(),
            "grep".into(),
        ],
    };

    CrushConfig {
        schema: "https://charm.land/crush.json".into(),
        providers,
        models,
        options: Some(options),
        permissions: Some(permissions),
    }
}

pub fn write_crush_config(config: &Config) -> anyhow::Result<PathBuf> {
    let path = crush_config_path();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let crush_config = build_crush_config(config);
    let json = serde_json::to_string_pretty(&crush_config)?;
    std::fs::write(&path, &json)?;

    info!("Crush config written to {}", path.display());
    Ok(path)
}

pub fn generate_crush_md(config: &Config) -> String {
    let devops = config
        .devops_model
        .as_deref()
        .unwrap_or("qwen3-coder:30b-gpu");
    let quick = config
        .quick_model
        .as_deref()
        .unwrap_or("devstral-small-2-gpu");
    format!(
        r#"# Local Ollama Development Environment

## Primary LLM

- **Large/Medium**: `{devops}`
- **Small/Quick**: `{quick}`
- **Provider**: Ollama Local at <http://localhost:{port}/v1/>
- **Embeddings**: `nomic-embed-text`
- **Qdrant**: <http://localhost:{qdrant}>

## Specialization

This environment is optimized for DevOps, Terraform, Ansible, YAML, JSON, TypeScript/JavaScript/Node.js, Go, Python, and Rust coding workflows.
"#,
        devops = devops,
        quick = quick,
        port = config.ollama_port,
        qdrant = config.qdrant_port,
    )
}

pub fn write_crush_md(config: &Config, project_root: &Path) -> anyhow::Result<PathBuf> {
    let path = project_root.join("CRUSH.md");
    let content = generate_crush_md(config);
    std::fs::write(&path, content)?;
    info!("CRUSH.md written to {}", path.display());
    Ok(path)
}
