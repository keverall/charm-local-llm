use crate::cli::{
    Cli, Command, CrushAction, CrushArgs, KiloAction, KiloArgs, ModelsAction, ModelsArgs,
    QdrantAction, QdrantArgs, ServiceAction, ServiceArgs, StartArgs, StatusArgs, StopArgs,
};
use crate::ollama::{ollama_running, start_ollama_direct, OllamaClient};
use crate::platform::{
    check_nvidia_smi, detect_platform, detect_platform_env_path, find_docker_compose,
    find_ollama_bin, load_env_file, resolve_project_root,
};
use crate::{Config, Platform};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;
use tracing::{info, warn};

fn run_systemctl(config: &Config, args: &[&str]) -> anyhow::Result<bool> {
    // Skip systemctl on non-Linux platforms
    if !matches!(config.platform, Platform::CachyOS | Platform::Linux) {
        return Ok(false);
    }

    // First try passwordless sudo if not root
    let output = if unsafe { libc::geteuid() } == 0 {
        ProcessCommand::new("systemctl").args(args).status()
    } else {
        ProcessCommand::new("sudo")
            .args(["-n", "systemctl"])
            .args(args)
            .status()
    };

    if output.map(|s| s.success()).unwrap_or(false) {
        return Ok(true);
    }

    // If sudo failed or systemd unavailable, try direct systemctl
    if !args.is_empty() && args[0] != "daemon-reload" {
        // For commands that matter (start/stop/restart), warn but don't error
        warn!("systemctl {:?} failed or requires passwordless sudo - falling back to direct ollama serve", args);
    }

    Ok(false)
}

pub async fn execute(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Start(args) => start(args, cli.verbose).await,
        Command::Stop(args) => stop(args, cli.verbose).await,
        Command::Status(args) => status(args, cli.verbose).await,
        Command::Models(args) => models(args, cli.verbose).await,
        Command::Service(args) => service(args, cli.verbose).await,
        Command::Qdrant(args) => qdrant(args, cli.verbose).await,
        Command::Crush(args) => crush(args, cli.verbose).await,
        Command::Kilo(args) => kilo(args, cli.verbose).await,
    }
}

async fn start(args: StartArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(args.project_root);
    let platform = detect_platform(args.platform_override.as_deref());
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let mut config = Config::new(platform, &project_root);
    config = config.with_env_overrides(env_vars);

    println!("🚀 Starting Ollama DevOps Environment...");
    println!("   Platform: {}", config.platform);
    println!("   Project root: {:?}", config.project_root);
    println!(
        "   Ollama bin: {:?}",
        find_ollama_bin().unwrap_or_else(|| PathBuf::from("ollama"))
    );

    if args.dry_run {
        println!(
            "   [dry-run] Would export OLLAMA_HOST={}",
            config.ollama_host
        );
        if matches!(config.platform, Platform::CachyOS | Platform::Linux) {
            println!(
                "   [dry-run] Would run: sudo -n systemctl start ollama (fallback: ollama serve)"
            );
        } else {
            println!("   [dry-run] Would run: ollama serve &");
        }
        return Ok(());
    }

    if find_ollama_bin().is_none() && config.ollama_bin == "ollama" {
        anyhow::bail!("Ollama binary not found. Please install Ollama first.");
    }

    if let Platform::CachyOS | Platform::Linux = config.platform {
        if let Some(gpu_info) = check_nvidia_smi() {
            println!("   GPU Status: {}", gpu_info);
        } else {
            println!("   nvidia-smi not found. GPU optimizations may not be available.");
        }
    }

    println!("🛑 Stopping existing Ollama processes...");

    // Try systemctl stop with passwordless sudo, silently skip if not available
    if matches!(config.platform, Platform::CachyOS | Platform::Linux) {
        let _ = run_systemctl(&config, &["stop", "ollama"]);
        let _ = run_systemctl(&config, &["daemon-reload"]);
    }

    let _ = ProcessCommand::new("pkill")
        .args(["-f", "ollama serve"])
        .output();

    std::thread::sleep(Duration::from_secs(2));
    println!("✅ Previous Ollama instances stopped.");

    println!("🚀 Starting Ollama server...");
    let log_dir = config.project_root.join("logs");
    std::fs::create_dir_all(&log_dir).ok();
    let log_file = log_dir.join(format!(
        "ollama-server-{}.log",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    ));

    let started_via_systemd = if matches!(config.platform, Platform::CachyOS | Platform::Linux) {
        let systemd_works = run_systemctl(&config, &["start", "ollama"])?;

        if systemd_works {
            tokio::time::sleep(Duration::from_secs(3)).await;

            let port = config.ollama_port;
            let running = tokio::task::spawn_blocking(move || ollama_running(port))
                .await
                .unwrap_or(false);

            if running {
                true
            } else {
                warn!("systemctl start failed to start Ollama, falling back to direct start");
                let cfg = config.clone();
                let lf = log_file.clone();
                tokio::task::spawn_blocking(move || start_ollama_direct(&cfg, &lf)).await??;
                false
            }
        } else {
            let cfg = config.clone();
            let lf = log_file.clone();
            tokio::task::spawn_blocking(move || start_ollama_direct(&cfg, &lf)).await??;
            false
        }
    } else {
        let cfg = config.clone();
        let lf = log_file.clone();
        tokio::task::spawn_blocking(move || start_ollama_direct(&cfg, &lf)).await??;
        false
    };

    if !started_via_systemd {
        let _ = install_systemd_env(&config);
    }

    for attempt in 1..16 {
        let port = config.ollama_port;
        let running = tokio::task::spawn_blocking(move || ollama_running(port))
            .await
            .unwrap_or(false);
        if running {
            info!("Ollama is ready (attempt {})", attempt);
            break;
        }
        if attempt == 15 {
            anyhow::bail!("Ollama failed to start after 30 seconds");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("🔍 Testing API connectivity...");
    for addr in &["127.0.0.1", "[::1]", "localhost"] {
        let url = format!("http://{}:{}/api/tags", addr, config.ollama_port);
        let url_clone = url.clone();
        let result = tokio::task::spawn_blocking(move || {
            reqwest::blocking::get(&url_clone)
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        })
        .await
        .unwrap_or(false);
        if result {
            println!("  {}: ✅ OK", url);
        } else {
            println!("  {}: ❌ FAIL", url);
        }
    }

    println!("📦 Ensuring models are present...");
    let client = OllamaClient::new(config.clone());

    let mut models_to_check: Vec<(String, Option<String>)> = config
        .default_models
        .iter()
        .cloned()
        .map(|m| (m, None))
        .collect();

    if let Some(ref dm) = config.devops_model {
        if !models_to_check.iter().any(|(m, _)| m == dm) {
            let mf = get_modfile_for_model(dm, config.platform, &config.modfile_dir);
            models_to_check.push((dm.clone(), mf));
        }
    }

    if let Some(ref qm) = config.quick_model {
        if !models_to_check.iter().any(|(m, _)| m == qm) {
            let mf = get_modfile_for_model(qm, config.platform, &config.modfile_dir);
            models_to_check.push((qm.clone(), mf));
        }
    }

    for (model, modelfile) in models_to_check {
        if let Err(e) = client.ensure_model(&model, modelfile.as_deref()).await {
            warn!("Failed to ensure model '{}': {}", model, e);
        }
    }

    println!("🔥 Warming up models...");
    let _ = warmup_models(&client, &config).await;

    if find_docker_compose().is_some() {
        println!("🐳 Starting Qdrant...");
        let _ = start_qdrant(&config).await;
    }

    println!("🔧 Configuring coding assistants...");
    match crate::crush::write_crush_config(&config) {
        Ok(path) => println!("   ✅ Crush config: {}", path.display()),
        Err(e) => warn!("   Failed to write Crush config: {}", e),
    }
    match crate::crush::write_crush_md(&config, &config.project_root) {
        Ok(path) => println!("   ✅ CRUSH.md: {}", path.display()),
        Err(e) => warn!("   Failed to write CRUSH.md: {}", e),
    }
    match crate::kilo_integration::patch_kilo_indexing(&config) {
        Ok(true) => println!("   ✅ Kilo config cleaned up (removed unsupported indexing block)"),
        Ok(false) => println!("   ✅ Kilo config has no unsupported indexing block"),
        Err(e) => warn!("   Failed to patch Kilo config: {}", e),
    }
    match crate::kilo_integration::write_agents_md(&config, &config.project_root) {
        Ok(path) => println!("   ✅ AGENTS.md: {}", path.display()),
        Err(e) => warn!("   Failed to write AGENTS.md: {}", e),
    }

    reload_vscode_window();

    print_final_status(&client).await;

    println!("✅✅✅ Environment Started Successfully ✅✅✅");

    Ok(())
}

async fn stop(args: StopArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(None);
    let platform = detect_platform(args.platform_override.as_deref());
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let config = Config::new(platform, &project_root).with_env_overrides(env_vars);

    println!("🛑 Shutting Down Ollama DevOps Environment...");

    if args.dry_run {
        println!("   [dry-run] Would stop docker containers");
        println!("   [dry-run] Would run: systemctl stop ollama");
        return Ok(());
    }

    if let Some(docker) = find_docker_compose() {
        let compose = config.project_root.join("docker-compose.yml");
        if compose.exists() {
            let _ = ProcessCommand::new(&docker)
                .args(["-f", compose.to_str().unwrap(), "down", "--remove-orphans"])
                .status();
            let _ = ProcessCommand::new("docker")
                .args(["container", "prune", "-f"])
                .status();
        }
        println!("🐳 Docker containers stopped.");
    }

    match platform {
        Platform::MacOS => {
            let _ = ProcessCommand::new("pkill").args(["-f", "ollama"]).status();
            std::thread::sleep(Duration::from_secs(2));
            let _ = ProcessCommand::new("pkill")
                .args(["-9", "-f", "ollama"])
                .status();
        }
        Platform::CachyOS | Platform::Linux => {
            let _ = run_systemctl(&config, &["disable", "ollama"]);
            let _ = run_systemctl(&config, &["stop", "ollama"]);
            std::thread::sleep(Duration::from_secs(2));
            let _ = ProcessCommand::new("pkill")
                .args(["-TERM", "-f", "ollama"])
                .status();
            std::thread::sleep(Duration::from_secs(2));
            let _ = ProcessCommand::new("pkill")
                .args(["-9", "-f", "ollama"])
                .status();
        }
        _ => {
            let _ = ProcessCommand::new("pkill").args(["-f", "ollama"]).status();
        }
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
    let port = config.ollama_port;
    let running = tokio::task::spawn_blocking(move || ollama_running(port))
        .await
        .unwrap_or(false);
    if running {
        warn!("Some Ollama processes may still be running");
    } else {
        println!("✅ Ollama processes stopped.");
    }

    println!("🧹 Cleanup complete.");
    println!("✅ Environment shutdown complete.");

    Ok(())
}

async fn status(_args: StatusArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(None);
    let platform = detect_platform(None);
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let config = Config::new(platform, &project_root).with_env_overrides(env_vars);

    let client = OllamaClient::new(config.clone());

    println!("📊 Ollama DevOps Environment Status");
    println!("================================");
    println!("Ollama API: {}", config.ollama_base_url);

    let models = client.list_models().await.unwrap_or_default();
    if models.is_empty() {
        println!("Available Models: (none)");
    } else {
        println!("Available Models:");
        for m in &models {
            println!("  - {} ({})", m.name, m.size);
        }
    }

    let running = client.list_running_models().await.unwrap_or_default();
    if running.is_empty() {
        println!("Loaded Models: (none)");
    } else {
        println!("Loaded Models:");
        for m in &running {
            println!("  - {} (VRAM: {} bytes)", m.name, m.size_vram);
        }
    }

    if matches!(config.platform, Platform::CachyOS | Platform::Linux) {
        if let Some(gpu) = check_nvidia_smi() {
            println!("GPU: {}", gpu);
        } else {
            println!("GPU: not detected");
        }
    }

    Ok(())
}

async fn models(args: ModelsArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(None);
    let platform = detect_platform(None);
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let config = Config::new(platform, &project_root).with_env_overrides(env_vars);
    let client = OllamaClient::new(config.clone());

    match args.action {
        ModelsAction::List => {
            let models: Vec<crate::ollama::ModelInfo> = client.list_models().await?;
            for m in models {
                println!("{} {} {}", m.name, m.size, m.modified_at);
            }
        }
        ModelsAction::Ensure { model } => {
            let mf = get_modfile_for_model(&model, config.platform, &config.modfile_dir);
            client.ensure_model(&model, mf.as_deref()).await?;
            println!("✅ Model '{}' ensured", model);
        }
        ModelsAction::Remove { model } => {
            client.remove_model(&model).await?;
            println!("✅ Model '{}' removed", model);
        }
    }

    Ok(())
}

async fn service(args: ServiceArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(None);
    let platform = detect_platform(None);
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let config = Config::new(platform, &project_root).with_env_overrides(env_vars);

    if !matches!(config.platform, Platform::CachyOS | Platform::Linux) {
        println!("Service management only available on Linux (CachyOS)");
        return Ok(());
    }

    match args.action {
        ServiceAction::Start => {
            if args.dry_run {
                println!("[dry-run] Would run: systemctl start ollama");
                return Ok(());
            }
            let status = ProcessCommand::new("systemctl")
                .args(["start", "ollama"])
                .status()?;
            if status.success() {
                println!("✅ Ollama service started");
            } else {
                println!("❌ Failed to start Ollama service");
            }
        }
        ServiceAction::Stop => {
            if args.dry_run {
                println!("[dry-run] Would run: systemctl stop ollama");
                return Ok(());
            }
            let status = ProcessCommand::new("systemctl")
                .args(["stop", "ollama"])
                .status()?;
            if status.success() {
                println!("✅ Ollama service stopped");
            } else {
                println!("❌ Failed to stop Ollama service");
            }
        }
        ServiceAction::Restart => {
            if args.dry_run {
                println!("[dry-run] Would run: systemctl restart ollama");
                return Ok(());
            }
            let status = ProcessCommand::new("systemctl")
                .args(["restart", "ollama"])
                .status()?;
            if status.success() {
                println!("✅ Ollama service restarted");
            } else {
                println!("❌ Failed to restart Ollama service");
            }
        }
        ServiceAction::Status => {
            let output = ProcessCommand::new("systemctl")
                .args(["is-active", "ollama"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()?;
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    Ok(())
}

async fn qdrant(args: QdrantArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(None);
    let platform = detect_platform(None);
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let config = Config::new(platform, &project_root).with_env_overrides(env_vars);

    let docker =
        find_docker_compose().ok_or_else(|| anyhow::anyhow!("docker-compose not found"))?;
    let compose = config.project_root.join("docker-compose.yml");

    if args.dry_run {
        match args.action {
            QdrantAction::Start => println!("[dry-run] Would run: docker-compose up -d"),
            QdrantAction::Stop => {
                println!("[dry-run] Would run: docker-compose down --remove-orphans")
            }
            QdrantAction::Status => println!("[dry-run] Would run: docker-compose ps"),
        }
        return Ok(());
    }

    if !compose.exists() {
        anyhow::bail!("docker-compose.yml not found at {:?}", compose);
    }

    use std::fs;
    let data_dir = &config.qdrant_data_dir;
    fs::create_dir_all(data_dir).ok();

    match args.action {
        QdrantAction::Start => {
            let _ = ProcessCommand::new("docker")
                .args(["rm", "-f", "qdrant"])
                .output();

            let status = ProcessCommand::new(&docker)
                .args(["-f", compose.to_str().unwrap(), "up", "-d"])
                .env("QDRANT_PORT", config.qdrant_port.to_string())
                .env(
                    "QDRANT_DATA_DIR",
                    data_dir.to_str().unwrap_or("./data/qdrant"),
                )
                .status()?;

            if status.success() {
                println!("✅ Qdrant containers started!");
                let qdrant_url = format!("http://localhost:{}/healthz", config.qdrant_port);
                for _ in 0..30 {
                    if reqwest::get(&qdrant_url)
                        .await
                        .map(|r| r.status().is_success())
                        .unwrap_or(false)
                    {
                        println!("✅ Qdrant is ready.");
                        return Ok(());
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
        QdrantAction::Stop => {
            let status = ProcessCommand::new(&docker)
                .args(["-f", compose.to_str().unwrap(), "down", "--remove-orphans"])
                .status()?;
            if status.success() {
                println!("✅ Qdrant stopped.");
            } else {
                println!("❌ Failed to stop Qdrant");
            }
        }
        QdrantAction::Status => {
            let output = ProcessCommand::new(&docker)
                .args(["-f", compose.to_str().unwrap(), "ps"])
                .output()?;
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    Ok(())
}

// Helpers

fn install_systemd_env(config: &Config) -> anyhow::Result<()> {
    if std::env::consts::OS != "linux" {
        return Ok(());
    }

    let env_file = if std::path::Path::new("/etc/os-release").exists() {
        let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        if content.to_lowercase().contains("cachyos") || content.to_lowercase().contains("arch") {
            std::path::PathBuf::from("/etc/sysconfig/ollama")
        } else {
            std::path::PathBuf::from("/etc/default/ollama")
        }
    } else {
        std::path::PathBuf::from("/etc/default/ollama")
    };

    let env_content = format!(
        "# Ollama environment configuration\n# Generated by charm on {}\n# Platform: {}\n\nOLLAMA_HOST={}\nOLLAMA_PORT={}\nOLLAMA_NUM_PARALLEL={}\nOLLAMA_MAX_LOADED_MODELS={}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
        config.platform,
        config.ollama_host,
        config.ollama_port,
        config.ollama_num_parallel,
        config.ollama_max_loaded_models,
    );

    let final_content = if let Some(mp) = &config.ollama_models_path {
        env_content + &format!("OLLAMA_MODELS={}\n", mp.display())
    } else {
        env_content
    };

    // Try to write with sudo if not root
    let is_root = unsafe { libc::geteuid() } == 0;
    let write_result = if is_root {
        std::fs::write(&env_file, final_content.clone())
    } else {
        // Use sudo tee to write the file
        let mut cmd = ProcessCommand::new("sudo");
        cmd.args([
            "-n",
            "tee",
            env_file.to_str().unwrap_or("/etc/default/ollama"),
        ]);
        use std::io::Write;
        if let Ok(mut child) = cmd.stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(final_content.as_bytes());
            }
            let _ = child.wait();
        }
        Ok(())
    };

    if let Err(e) = write_result {
        warn!(
            "Could not write systemd env file (requires root/sudo): {}",
            e
        );
    }

    let _ = ProcessCommand::new("sudo")
        .args(["-n", "systemctl", "daemon-reload"])
        .output();
    Ok(())
}

async fn warmup_models(client: &OllamaClient, config: &Config) -> anyhow::Result<()> {
    let mut models_to_warm: Vec<String> = if let Some(dm) = &config.devops_model {
        vec![dm.clone()]
    } else {
        vec![]
    };
    if let Some(qm) = &config.quick_model {
        if !models_to_warm.contains(qm) {
            models_to_warm.push(qm.clone());
        }
    }
    for m in &config.default_models {
        if (m.contains("30b") || m.contains("26b")) && !models_to_warm.contains(m) {
            models_to_warm.push(m.clone());
        }
    }
    for model in models_to_warm {
        let timeout = if model.contains("30b") || model.contains("26b") {
            300
        } else {
            120
        };
        let _ = client.warmup_model(&model, timeout).await;
    }

    Ok(())
}

async fn start_qdrant(config: &Config) -> anyhow::Result<()> {
    use std::fs;
    let data_dir = &config.qdrant_data_dir;
    fs::create_dir_all(data_dir).ok();

    let docker =
        find_docker_compose().ok_or_else(|| anyhow::anyhow!("docker-compose not found"))?;
    let compose = config.project_root.join("docker-compose.yml");

    let _ = ProcessCommand::new("docker")
        .args(["rm", "-f", "qdrant"])
        .output();

    let status = ProcessCommand::new(&docker)
        .args(["-f", compose.to_str().unwrap(), "up", "-d"])
        .env("QDRANT_PORT", config.qdrant_port.to_string())
        .env(
            "QDRANT_DATA_DIR",
            data_dir.to_str().unwrap_or("./data/qdrant"),
        )
        .status()?;

    if status.success() {
        println!("✅ Qdrant containers started!");
        let qdrant_url = format!("http://localhost:{}/healthz", config.qdrant_port);
        for _ in 0..30 {
            if reqwest::get(&qdrant_url)
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                println!("✅ Qdrant is ready.");
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(())
}

async fn print_final_status(client: &OllamaClient) {
    println!();
    println!("📊 Phase 7: Environment Status");
    println!("==========================");

    if let Ok(models) = client.list_models().await {
        println!("Available Models:");
        if models.is_empty() {
            println!("  Could not list models");
        } else {
            for m in models {
                println!("  - {} ({})", m.name, m.size);
            }
        }
    }

    if let Ok(running) = client.list_running_models().await {
        println!("Loaded Models:");
        if running.is_empty() {
            println!("  Could not list running models");
        } else {
            for m in running {
                println!("  - {} (VRAM: {} bytes)", m.name, m.size_vram);
            }
        }
    }
}

fn get_modfile_for_model(model: &str, platform: Platform, dir: &Path) -> Option<String> {
    if !dir.exists() {
        return None;
    }

    match platform {
        Platform::MacOS => {
            let candidates = vec![
                format!("modfile-{}", model.replace([':', '.'], "-")),
                format!(
                    "modfile-{}",
                    if model.contains("qwen") {
                        "qwen-devops"
                    } else if model.contains("gemma") {
                        "gemma4"
                    } else {
                        model
                    }
                ),
            ];
            for c in candidates {
                let p = dir.join(&c);
                if p.exists() {
                    return Some(c);
                }
            }
        }
        Platform::CachyOS | Platform::Linux => {
            let candidates: Vec<String> = match model {
                "qwen3-coder:30b-gpu" => vec!["qwen3-coder-30b-gpu.modelfile".into()],
                "gemma4:26b-devops" => vec!["gemma4-26b-devops.modelfile".into()],
                "devstral-small-2-gpu" => vec!["devstral-small-2-gpu.modelfile".into()],
                "qwen3:8b" => vec!["qwen3-8b-gpu.modelfile".into()],
                "nomic-embed-text:latest" | "nomic-embed-text" => {
                    vec!["nomic-embed-text-GPU.modelfile".into()]
                }
                _ => {
                    let base = model.replace([':', '.'], "-");
                    vec![format!("{}.modelfile", base)]
                }
            };
            for c in candidates {
                let p = dir.join(&c);
                if p.exists() {
                    return Some(c);
                }
            }
        }
        _ => {}
    }

    None
}

fn reload_vscode_window() {
    println!("   💡 Reload VS Code window to pick up config changes (Ctrl+Shift+P → Developer: Reload Window)");
}

fn init_logging(verbose: bool) {
    use tracing_subscriber::EnvFilter;
    let filter = if verbose {
        EnvFilter::from_default_env().add_directive("charm_local_llm=debug".parse().unwrap())
    } else {
        EnvFilter::from_default_env().add_directive("charm_local_llm=info".parse().unwrap())
    };
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn crush(args: CrushArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(args.project_root);
    let platform = detect_platform(None);
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let config = Config::new(platform, &project_root).with_env_overrides(env_vars);

    match args.action {
        CrushAction::Init => {
            let path = crate::crush::write_crush_config(&config)?;
            println!("✅ Crush config written to {}", path.display());
            println!(
                "   Provider: ollama (http://localhost:{}/v1/)",
                config.ollama_port
            );
            println!(
                "   Primary: large → {}",
                config
                    .devops_model
                    .as_deref()
                    .unwrap_or("qwen3-coder:30b-gpu")
            );
            println!(
                "   Quick:   small → {}",
                config
                    .quick_model
                    .as_deref()
                    .unwrap_or("devstral-small-2-gpu")
            );
        }
        CrushAction::Status => {
            let path = crate::crush::crush_config_path();
            println!("📋 Crush Config Status");
            println!("======================");
            println!("Config path: {}", path.display());
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let config: serde_json::Value = serde_json::from_str(&content)?;
                println!("Config exists: ✅");
                if let Some(providers) = config.get("providers") {
                    println!("Providers: {}", serde_json::to_string_pretty(providers)?);
                }
                if let Some(models) = config.get("models") {
                    println!("Models: {}", serde_json::to_string_pretty(models)?);
                }
            } else {
                println!("Config exists: ❌ (run 'charm crush init' to create)");
            }
        }
        CrushAction::Context => {
            let path = crate::crush::write_crush_md(&config, &project_root)?;
            println!("✅ CRUSH.md written to {}", path.display());
        }
    }

    Ok(())
}

async fn kilo(args: KiloArgs, verbose: bool) -> anyhow::Result<()> {
    init_logging(verbose);

    let project_root = resolve_project_root(args.project_root);
    let platform = detect_platform(None);
    let env_path = detect_platform_env_path(&project_root, platform);
    let env_vars = load_env_file(&env_path);
    let config = Config::new(platform, &project_root).with_env_overrides(env_vars);

    match args.action {
        KiloAction::Init => {
            let changed = crate::kilo_integration::patch_kilo_indexing(&config)?;
            if changed {
                println!("✅ Kilo config cleaned up: removed unsupported indexing block");
            } else {
                println!("✅ Kilo config has no unsupported indexing block");
            }
        }
        KiloAction::Status => {
            let status = crate::kilo_integration::verify_kilo_config(&config)?;
            println!("📋 Kilo Config Status");
            println!("=====================");
            println!(
                "Config exists: {}",
                if status.config_exists { "✅" } else { "❌" }
            );
            println!(
                "Indexing block removed: {}",
                if status.indexing_configured {
                    "✅"
                } else {
                    "❌"
                }
            );
            if !status.issues.is_empty() {
                println!("Issues:");
                for issue in &status.issues {
                    println!("  ⚠️  {}", issue);
                }
            }
        }
        KiloAction::Context => {
            let path = crate::kilo_integration::write_agents_md(&config, &project_root)?;
            println!("✅ AGENTS.md written to {}", path.display());
        }
    }

    Ok(())
}
