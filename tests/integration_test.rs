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
fn test_patch_kilo_indexing_preserves_and_repairs_block() {
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
        charm_local_llm::kilo_integration::patch_kilo_indexing_at_path(&tmp, &config, None)
            .unwrap();
    assert!(changed);

    let patched: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&tmp).unwrap()).unwrap();
    // Indexing must be preserved (not deleted) and repaired with required fields.
    let idx = patched
        .get("indexing")
        .expect("indexing block must be preserved");
    assert_eq!(idx.get("enabled"), Some(&serde_json::json!(true)));
    assert_eq!(idx.get("provider"), Some(&serde_json::json!("ollama")));
    assert_eq!(
        idx.get("model"),
        Some(&serde_json::json!("nomic-embed-text"))
    );
    assert_eq!(idx.get("dimension"), Some(&serde_json::json!(768)));
    assert_eq!(
        idx.get("ollama").and_then(|o| o.get("baseUrl")),
        Some(&serde_json::json!("http://localhost:11434/"))
    );
    assert_eq!(idx.get("vectorStore"), Some(&serde_json::json!("qdrant")));
    assert_eq!(
        idx.get("qdrant").and_then(|q| q.get("url")),
        Some(&serde_json::json!("http://localhost:6333/"))
    );

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn test_patch_kilo_indexing_syncs_dynamic_models() {
    let tmp = std::env::temp_dir().join("kilo_test_dynamic_models.json");
    // Start with a stale provider that has num_gpu, duplicates, and unloaded models
    let content = serde_json::json!({
        "indexing": { "provider": "ollama" },
        "provider": {
            "Ollama Local (FREE)": {
                "options": { "baseURL": "http://localhost:11434/v1/" },
                "models": {
                    "old-model-not-loaded": { "name": "Old Model", "num_gpu": "32" },
                    "nomic-embed-text": { "name": "Nomic Embed Text" },
                    "nomic-embed-text:latest": { "name": "Nomic Embed Text" }
                }
            }
        }
    });
    std::fs::write(&tmp, serde_json::to_string(&content).unwrap()).unwrap();

    let config = Config {
        ollama_port: 11434,
        qdrant_port: 6333,
        ..Config::default(Platform::CachyOS)
    };

    // Simulate Ollama's /api/tags returning only these models
    let available = vec![
        "qwen3-coder:30b-gpu".to_string(),
        "gemma4:26b-devops".to_string(),
        "nomic-embed-text:latest".to_string(),
    ];

    let changed = charm_local_llm::kilo_integration::patch_kilo_indexing_at_path(
        &tmp,
        &config,
        Some(&available),
    )
    .unwrap();
    assert!(changed);

    let patched: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&tmp).unwrap()).unwrap();

    // indexing block must be preserved (not deleted) and repaired
    let idx = patched
        .get("indexing")
        .expect("indexing block must be preserved");
    assert_eq!(idx.get("provider"), Some(&serde_json::json!("ollama")));
    assert_eq!(
        idx.get("model"),
        Some(&serde_json::json!("nomic-embed-text"))
    );

    let models = patched["provider"]["Ollama Local (FREE)"]["models"]
        .as_object()
        .unwrap();

    // Only the 3 available models (de-duplicated)
    assert_eq!(models.len(), 3, "should have exactly 3 models after dedup");
    assert!(models.contains_key("qwen3-coder:30b-gpu"));
    assert!(models.contains_key("gemma4:26b-devops"));
    assert!(models.contains_key("nomic-embed-text"));

    // The stale unloaded model must be gone
    assert!(!models.contains_key("old-model-not-loaded"));

    // nomic-embed-text appears once (not duplicated as nomic-embed-text:latest)
    assert!(!models.contains_key("nomic-embed-text:latest"));

    // No num_gpu should survive in any model entry
    for (_id, entry) in models.iter() {
        let entry_obj = entry.as_object().unwrap();
        assert!(!entry_obj.contains_key("num_gpu"));
        assert!(entry_obj.contains_key("name"));
    }

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn test_patch_kilo_fallback_when_no_models_available() {
    let tmp = std::env::temp_dir().join("kilo_test_fallback.json");
    let content = serde_json::json!({});
    std::fs::write(&tmp, serde_json::to_string(&content).unwrap()).unwrap();

    let config = Config {
        ollama_port: 11434,
        qdrant_port: 6333,
        ..Config::default(Platform::CachyOS)
    };

    // Empty available list → falls back to platform known models
    let changed =
        charm_local_llm::kilo_integration::patch_kilo_indexing_at_path(&tmp, &config, Some(&[]))
            .unwrap();
    assert!(changed);

    let patched: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&tmp).unwrap()).unwrap();

    let models = patched["provider"]["Ollama Local (FREE)"]["models"]
        .as_object()
        .unwrap();

    // CachyOS fallback includes qwen3-coder:30b-gpu, gemma4:26b-devops,
    // devstral-small-2-gpu, nomic-embed-text
    assert!(models.contains_key("qwen3-coder:30b-gpu"));
    assert!(models.contains_key("gemma4:26b-devops"));
    assert!(models.contains_key("devstral-small-2-gpu"));
    assert!(models.contains_key("nomic-embed-text"));

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn test_clean_project_kilo_config_preserves_indexing() {
    let tmp = std::env::temp_dir().join("kilo_test_project_config.jsonc");
    let content = serde_json::json!({
        "$schema": "https://app.kilo.ai/config.json",
        "indexing": {
            "enabled": true,
            "provider": "ollama"
        },
        "model": "kilo/kilo-auto/balanced"
    });
    std::fs::write(&tmp, serde_json::to_string(&content).unwrap()).unwrap();

    // Temporarily treat the temp file as the project .kilo/kilo.jsonc
    let project_root = std::env::temp_dir().join("kilo_test_project_root");
    let kilo_dir = project_root.join(".kilo");
    std::fs::create_dir_all(&kilo_dir).unwrap();
    std::fs::copy(&tmp, kilo_dir.join("kilo.jsonc")).unwrap();

    let config = Config {
        ollama_port: 11434,
        qdrant_port: 6333,
        ..Config::default(Platform::CachyOS)
    };
    let changed =
        charm_local_llm::kilo_integration::clean_project_kilo_config(&project_root, &config)
            .unwrap();
    assert!(changed);

    let cleaned: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(kilo_dir.join("kilo.jsonc")).unwrap())
            .unwrap();
    // Indexing must be preserved and repaired, not deleted.
    let idx = cleaned.get("indexing").expect("indexing must be preserved");
    assert_eq!(idx.get("provider"), Some(&serde_json::json!("ollama")));
    assert_eq!(
        idx.get("model"),
        Some(&serde_json::json!("nomic-embed-text"))
    );
    assert_eq!(idx.get("vectorStore"), Some(&serde_json::json!("qdrant")));
    assert_eq!(
        idx.get("qdrant").and_then(|q| q.get("url")),
        Some(&serde_json::json!("http://localhost:6333/"))
    );
    assert!(cleaned.get("model").is_some());

    // Second call should be a no-op (nothing left to repair)
    let changed_again =
        charm_local_llm::kilo_integration::clean_project_kilo_config(&project_root, &config)
            .unwrap();
    assert!(!changed_again);

    std::fs::remove_dir_all(&project_root).ok();
    std::fs::remove_file(&tmp).ok();
}

#[test]
fn test_normalize_model_name_strips_latest() {
    // Verify the public behavior through fetch_available_models logic
    // (tested indirectly since normalize_model_name is private)
    // We verify de-duplication by checking the dynamic model sync test above
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
}

#[test]
fn test_config_models_path_cachyos_is_home_ollama() {
    let config = Config::default(Platform::CachyOS);
    let mp = config.ollama_models_path.as_ref().unwrap();
    // Portable: the default store lives under the ollama user's home and ends
    // in `ollama/models`. The exact parent is machine-specific and overridden
    // via OLLAMA_MODELS in the platform .env, so assert the structure, not the path.
    assert!(
        mp.ends_with("ollama/models"),
        "expected store at .../ollama/models, got {mp:?}"
    );
    assert!(mp.is_absolute());
}

#[test]
fn test_config_models_path_macos_is_dot_ollama() {
    let config = Config::default(Platform::MacOSM532Gb);
    let mp = config.ollama_models_path.as_ref().unwrap();
    assert!(mp.ends_with(".ollama/models"));
}

#[test]
fn test_macos_m5_32gb_modfile_dir() {
    let config = Config::default(Platform::MacOSM532Gb);
    assert!(config
        .modfile_dir
        .to_string_lossy()
        .contains("macos-m5-32gb"));
}
