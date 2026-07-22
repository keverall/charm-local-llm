use charm_local_llm::{Config, Platform};
use std::collections::HashMap;

#[test]
fn test_build_crush_config_has_ollama_provider() {
    let config = Config::default(Platform::CachyOS);
    let crush = charm_local_llm::crush::build_crush_config(&config);

    assert_eq!(crush.schema, "https://charm.land/crush.json");
    assert!(crush.providers.contains_key("ollama"));

    let ollama = &crush.providers["ollama"];
    assert_eq!(ollama.provider_type, "ollama");
    assert_eq!(ollama.base_url, "http://localhost:11434/v1/");
    assert!(ollama.discover_models);
}

#[test]
fn test_build_crush_config_models_point_to_ollama() {
    let config = Config::default(Platform::CachyOS);
    let crush = charm_local_llm::crush::build_crush_config(&config);

    assert_eq!(crush.models.len(), 3);
    for model in crush.models.values() {
        assert_eq!(model.provider, "ollama");
    }
}

#[test]
fn test_build_crush_config_large_is_devops_model() {
    let config = Config::default(Platform::CachyOS);
    let crush = charm_local_llm::crush::build_crush_config(&config);

    let large = &crush.models["large"];
    assert_eq!(large.model, "gemma4:26b-devops");
}

#[test]
fn test_build_crush_config_small_is_quick_model() {
    let config = Config::default(Platform::CachyOS);
    let crush = charm_local_llm::crush::build_crush_config(&config);

    let small = &crush.models["small"];
    assert_eq!(small.model, "devstral-small-2-gpu");
}

#[test]
fn test_build_crush_config_serializes_to_valid_json() {
    let config = Config::default(Platform::CachyOS);
    let crush = charm_local_llm::crush::build_crush_config(&config);
    let json = serde_json::to_string_pretty(&crush).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("providers").is_some());
    assert!(parsed.get("models").is_some());
}

#[test]
fn test_crush_config_path_is_under_home() {
    let path = charm_local_llm::crush::crush_config_path();
    assert!(path.to_string_lossy().contains(".config/crush/crush.json"));
}

#[test]
fn test_generate_crush_md_contains_devops_model() {
    let config = Config::default(Platform::CachyOS);
    let md = charm_local_llm::crush::generate_crush_md(&config);
    assert!(md.contains("gemma4:26b-devops"));
    assert!(md.contains("Primary LLM"));
    assert!(md.contains("localhost:11434"));
}

#[test]
fn test_generate_crush_md_contains_qdrant() {
    let config = Config::default(Platform::CachyOS);
    let md = charm_local_llm::crush::generate_crush_md(&config);
    assert!(md.contains("Qdrant"));
    assert!(md.contains("localhost:6333"));
}

#[test]
fn test_verify_kilo_config_missing_file() {
    let config = Config::default(Platform::CachyOS);
    let status = charm_local_llm::kilo_integration::verify_kilo_config_from_path(
        &std::path::PathBuf::from("/nonexistent/kilo.json"),
        &config,
    )
    .unwrap();
    assert!(!status.config_exists);
    assert!(!status.indexing_configured);
}

#[test]
fn test_verify_kilo_config_valid() {
    let tmp = std::env::temp_dir().join("kilo_test_valid.json");
    let content = serde_json::json!({
        "model": "kilo/kilo-auto/balanced"
    });
    std::fs::write(&tmp, serde_json::to_string(&content).unwrap()).unwrap();

    let config = Config::default(Platform::CachyOS);
    let status =
        charm_local_llm::kilo_integration::verify_kilo_config_from_path(&tmp, &config).unwrap();
    assert!(status.config_exists);
    assert!(status.indexing_configured);
    assert!(status.issues.is_empty());

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn test_verify_kilo_config_with_invalid_indexing() {
    let tmp = std::env::temp_dir().join("kilo_test_invalid_indexing.json");
    let content = serde_json::json!({
        "indexing": {
            "provider": "ollama",
            "ollama": { "baseUrl": "http://localhost:11434" }
        }
    });
    std::fs::write(&tmp, serde_json::to_string(&content).unwrap()).unwrap();

    let config = Config::default(Platform::CachyOS);
    let status =
        charm_local_llm::kilo_integration::verify_kilo_config_from_path(&tmp, &config).unwrap();
    assert!(status.config_exists);
    assert!(!status.indexing_configured);
    assert!(!status.issues.is_empty());

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn test_patch_kilo_indexing_removes_invalid_block() {
    let tmp = std::env::temp_dir().join("kilo_test_patch.json");
    let content = serde_json::json!({
        "indexing": {
            "provider": "ollama",
            "ollama": { "baseUrl": "http://localhost:11434" }
        }
    });
    std::fs::write(&tmp, serde_json::to_string(&content).unwrap()).unwrap();

    let config = Config {
        ollama_port: 11434,
        qdrant_port: 6333,
        ..Config::default(Platform::CachyOS)
    };
    let changed =
        charm_local_llm::kilo_integration::patch_kilo_indexing_at_path(&tmp, &config).unwrap();
    assert!(changed);

    let patched: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&tmp).unwrap()).unwrap();
    assert!(patched.get("indexing").is_none());

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn test_generate_agents_md_contains_project_info() {
    let config = Config::default(Platform::CachyOS);
    let md = charm_local_llm::kilo_integration::generate_agents_md(&config);
    assert!(md.contains("charm-local-llm"));
    assert!(md.contains("gemma4:26b-devops"));
    assert!(md.contains("devstral-small-2-gpu"));
    assert!(md.contains("make build"));
}

#[test]
fn test_macos_m4_24gb_defaults_to_14b_devops() {
    let config = Config::default(Platform::MacOSM424Gb);
    assert_eq!(
        config.devops_model.as_deref(),
        Some("qwen2.5-coder:14b-devops")
    );
    assert_eq!(
        config.quick_model.as_deref(),
        Some("qwen2.5-coder:7b-quick")
    );
    assert!(config
        .modfile_dir
        .to_string_lossy()
        .contains("macos-m4-24gb"));
}

#[test]
fn test_config_evaluation_route_applies_env_overrides() {
    let root = std::path::PathBuf::from("/tmp/kcharm-eval-test");
    let mut env = HashMap::new();
    env.insert("OLLAMA_NUM_PARALLEL".to_string(), "8".to_string());
    env.insert("OLLAMA_MAX_LOADED_MODELS".to_string(), "1".to_string());
    env.insert("OLLAMA_GPU_LAYERS".to_string(), "40".to_string());
    env.insert("DEVOPS_MODEL".to_string(), "custom-devops".to_string());

    let config = Config::new(Platform::CachyOS, &root).with_env_overrides(env);

    assert_eq!(config.ollama_num_parallel, 8);
    assert_eq!(config.ollama_max_loaded_models, 1);
    assert_eq!(config.ollama_gpu_layers, Some(40));
    assert_eq!(config.devops_model.as_deref(), Some("custom-devops"));
}

#[test]
fn test_cachyos_default_passes_single_gpu_profile() {
    let config = Config::default(Platform::CachyOS);
    assert!(config.validate_cachyos_single_gpu_profile().is_ok());
}

#[test]
fn test_cachyos_profile_rejects_unset_gpu_layers() {
    let root = std::path::PathBuf::from("/tmp/kcharm-eval-test");
    let mut env = HashMap::new();
    env.insert("OLLAMA_GPU_LAYERS".to_string(), "0".to_string());

    let config = Config::new(Platform::CachyOS, &root).with_env_overrides(env);
    assert!(config.validate_cachyos_single_gpu_profile().is_err());
}

#[test]
fn test_cachyos_profile_rejects_zero_parallel() {
    let root = std::path::PathBuf::from("/tmp/kcharm-eval-test");
    let mut env = HashMap::new();
    env.insert("OLLAMA_NUM_PARALLEL".to_string(), "0".to_string());

    let config = Config::new(Platform::CachyOS, &root).with_env_overrides(env);
    assert!(config.validate_cachyos_single_gpu_profile().is_err());
}

#[test]
fn test_macos_m5_32gb_defaults_to_27b_devops() {
    let config = Config::default(Platform::MacOSM532Gb);
    assert_eq!(
        config.devops_model.as_deref(),
        Some("qwen3.6:27b-instruct-q4_K_M-devops")
    );
    assert_eq!(
        config.quick_model.as_deref(),
        Some("qwen2.5-coder:14b-quick")
    );
    assert!(config
        .modfile_dir
        .to_string_lossy()
        .contains("macos-m5-32gb"));
}
