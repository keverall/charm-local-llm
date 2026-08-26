use crate::Platform;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::Result;

pub fn detect_platform(override_str: Option<&str>) -> Platform {
    if let Some(o) = override_str {
        if o != "auto" {
            return Platform::from_string(o);
        }
    }

    if std::env::consts::OS == "macos" {
        return detect_mac_variant();
    }

    if std::path::Path::new("/etc/os-release").exists() {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            let lower = content.to_lowercase();
            if lower.contains("cachyos") || lower.contains("arch") {
                return Platform::CachyOS;
            }
        }
    }

    if std::env::consts::OS == "linux" {
        return Platform::Linux;
    }

    Platform::Unknown
}

fn detect_mac_variant() -> Platform {
    let chip = detect_apple_silicon_chip();
    let ram_gb = detect_system_ram_gb();

    match (chip, ram_gb) {
        (AppleChip::M4, ram) if ram <= 24 => Platform::MacOSM424Gb,
        (AppleChip::M4, _) => Platform::MacOSM432Gb,
        (AppleChip::M5, ram) if ram <= 24 => Platform::MacOSM524Gb,
        (AppleChip::M5, _) => Platform::MacOSM532Gb,
        (AppleChip::Unknown, ram) if ram <= 24 => Platform::MacOSM424Gb,
        (AppleChip::Unknown, _) => Platform::MacOSM432Gb,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppleChip {
    M4,
    M5,
    Unknown,
}

fn detect_apple_silicon_chip() -> AppleChip {
    let Ok(output) = Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
    else {
        return AppleChip::Unknown;
    };

    if !output.status.success() {
        return AppleChip::Unknown;
    }

    let brand = String::from_utf8_lossy(&output.stdout).to_lowercase();

    if brand.contains("m5") {
        AppleChip::M5
    } else if brand.contains("m4") {
        AppleChip::M4
    } else {
        AppleChip::Unknown
    }
}

fn detect_system_ram_gb() -> u64 {
    let Ok(output) = Command::new("sysctl").args(["-n", "hw.memsize"]).output() else {
        return 24;
    };

    if !output.status.success() {
        return 24;
    }

    let bytes_str = String::from_utf8_lossy(&output.stdout);
    let bytes: u64 = bytes_str.trim().parse().unwrap_or(24 * 1024 * 1024 * 1024);
    bytes / (1024 * 1024 * 1024)
}

pub fn find_ollama_bin() -> Option<PathBuf> {
    which::which("ollama").ok()
}

pub fn find_docker_compose() -> Option<PathBuf> {
    which::which("docker-compose")
        .ok()
        .or_else(|| which::which("docker").ok())
}

/// Resolve the compose CLI as a program plus leading arguments. Handles both
/// the standalone `docker-compose` binary and the `docker compose` plugin
/// (where `compose` must be passed as the first argument).
pub fn compose_command() -> Option<(PathBuf, Vec<String>)> {
    if let Ok(bin) = which::which("docker-compose") {
        return Some((bin, Vec::new()));
    }
    which::which("docker")
        .ok()
        .map(|bin| (bin, vec!["compose".to_string()]))
}

pub fn check_nvidia_smi() -> Option<String> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,driver_version,memory.total,memory.used,memory.free",
            "--format=csv,noheader",
        ])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Some(stdout.trim().to_string());
    }

    None
}

/// Parse a single `.env` value: strip a trailing inline `#` comment (only when
/// the `#` is outside quotes and preceded by whitespace) and surrounding
/// double quotes. Without this, a value like `"50"   # comment` leaks the quote
/// and comment into the resolved value, producing a broken
/// `OLLAMA_KEEP_ALIVE=2m"   # ...` line in /etc/default/ollama.
fn parse_env_value(raw: &str) -> String {
    let mut in_quotes = false;
    let mut comment_at: Option<usize> = None;
    for (i, c) in raw.char_indices() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == '#' && !in_quotes {
            let prev_ws = i == 0
                || raw[..i]
                    .chars()
                    .next_back()
                    .map(|p| p.is_whitespace())
                    .unwrap_or(true);
            if prev_ws {
                comment_at = Some(i);
                break;
            }
        }
    }
    let trimmed = match comment_at {
        Some(i) => &raw[..i],
        None => raw,
    };
    trimmed.trim().trim_matches('"').to_string()
}

pub fn load_env_file(path: &PathBuf) -> HashMap<String, String> {
    let mut env = HashMap::new();
    if !path.exists() {
        return env;
    }

    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = parse_env_value(value);
                env.insert(key.into(), value);
            }
        }
    }

    env
}

pub fn detect_platform_env_path(project_root: &Path, platform: Platform) -> PathBuf {
    project_root
        .join("platform")
        .join(platform.platform_dir())
        .join(".env")
}

/// Path of the file that records the kcharm project root, written during
/// `kcharm service install`. This is what makes non-interactive boot/login
/// starts (systemd user unit, XDG autostart) resolve the repo correctly even
/// though their working directory is `$HOME`.
pub fn project_root_marker_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("kcharm")
        .join("project-root")
}

/// True only when `dir` is the actual charm-local-llm checkout.
///
/// A loose check (any dir with a `docker-compose.yml`) caused `kcharm` to
/// hijack unrelated repos that happen to contain a compose file (e.g. a
/// sibling `image-build-automation` checkout), running their containers
/// instead of this repo's Qdrant stack. We now require the real signature:
/// a `Cargo.toml` whose package is `charm-local-llm` plus the entry point.
fn is_charm_repo(dir: &Path) -> bool {
    let cargo = dir.join("Cargo.toml");
    if !cargo.exists() || !dir.join("src").join("main.rs").exists() {
        return false;
    }
    match std::fs::read_to_string(&cargo) {
        Ok(content) => content.contains("name = \"charm-local-llm\""),
        Err(_) => false,
    }
}

/// Locate the charm-local-llm checkout by walking up from the running
/// executable, so the project root resolves correctly regardless of the
/// current working directory.
fn charm_repo_from_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    exe_dir
        .ancestors()
        .find(|dir| is_charm_repo(dir))
        .map(|dir| dir.to_path_buf())
}

/// Persist the resolved project root so boot-time runs can find it.
pub fn save_project_root(root: &Path) -> std::io::Result<()> {
    let marker = project_root_marker_path();
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(marker, format!("{}\n", root.display()))
}

fn marker_project_root() -> Option<PathBuf> {
    let content = std::fs::read_to_string(project_root_marker_path()).ok()?;
    let path = PathBuf::from(content.trim());
    if is_charm_repo(&path) {
        Some(path)
    } else {
        None
    }
}

pub fn resolve_project_root(override_path: Option<PathBuf>) -> PathBuf {
    if let Some(p) = override_path {
        let canonical = std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from("."));
        return canonical;
    }

    // Explicit environment override (used by the systemd user unit).
    if let Ok(env_root) = std::env::var("KCHARM_PROJECT_ROOT") {
        let path = PathBuf::from(env_root);
    if is_charm_repo(&path) {
            return path;
        }
    }

    // Resolve from the running executable — this works no matter what the
    // current directory is, so `kcharm` always targets this repo.
    if let Some(root) = charm_repo_from_exe() {
        return root;
    }

    // Walk up from the current directory looking for the checkout.
    if let Ok(current) = std::env::current_dir() {
        for dir in current.ancestors() {
            if is_charm_repo(dir) {
                return dir.to_path_buf();
            }
        }
    }

    // Recorded at bootstrap time — the path used by login/boot starts.
    if let Some(root) = marker_project_root() {
        return root;
    }

    // Last resort: the executable-relative repo, else the current directory.
    charm_repo_from_exe().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    })
}

/// Ensure the Docker daemon is running, starting it via systemd if needed.
/// Returns true when Docker responds to `docker info`.
pub fn ensure_docker_running() -> bool {
    let docker_ok = || {
        Command::new("docker")
            .arg("info")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    if docker_ok() {
        return true;
    }

    if std::env::consts::OS != "linux" {
        return false;
    }

    // Try to start the daemon (passwordless sudo, or root).
    let started = if unsafe { libc::geteuid() } == 0 {
        Command::new("systemctl")
            .args(["start", "docker"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        Command::new("sudo")
            .args(["-n", "systemctl", "start", "docker"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    if !started {
        return false;
    }

    // Wait for the daemon socket to accept commands.
    for _ in 0..30 {
        if docker_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    false
}

/// Best-effort check that the Ollama model registry is reachable. Every
/// `ensure_model` pull depends on `registry.ollama.ai`, so if this host is
/// unreachable there is no point attempting pulls — they would fail and
/// (historically) be swallowed, leaving DEFAULT_MODELS silently missing.
pub async fn registry_reachable() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .unwrap_or_default();

    // `GET /v2/` returns 404/401 unless authenticated, so it is NOT a
    // reliable reachability probe. Instead hit a manifest endpoint under
    // `/v2/library/<image>/manifests/<tag>` which returns 200 when the
    // registry is reachable (and any non-network error is treated as
    // unreachable).
    let url = "https://registry.ollama.ai/v2/library/qwen3-coder/manifests/latest";
    client
        .get(url)
        .send()
        .await
        .map(|r| r.status().is_success() || r.status().is_redirection())
        .unwrap_or(false)
}

/// Wait up to `max_wait_secs` for the Ollama model registry to become
/// reachable. If it never does, alert the user with a desktop popup and return
/// an error so the caller aborts `kcharm start` loudly (exit 1) instead of
/// producing a half-populated store. The popup is best-effort (it needs a
/// running session); the error is always surfaced to the journal/terminal too.
pub async fn wait_for_registry_or_alert(max_wait_secs: u64) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < max_wait_secs {
        if registry_reachable().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    // Final attempt in case the loop exited immediately after a check.
    if registry_reachable().await {
        return Ok(());
    }

    let title = "kcharm: no internet — models not loaded";
    let body = "kcharm could not reach the Ollama model registry, so the models in \
DEFAULT_MODELS were NOT pulled. Your local store will be missing models. \
Connect to the internet and re-run `make sod` (or reboot).";
    desktop_notify(title, body);
    anyhow::bail!(
        "No connectivity to the Ollama model registry after {}s. Aborting start so models are \
         not silently missing. Connect to the internet and re-run `make sod` (or reboot).",
        max_wait_secs
    )
}

/// Show a desktop notification popup (best-effort). Uses the platform-native
/// mechanism: notify-send on Linux, osascript on macOS. Failures are ignored —
/// the message is always also surfaced to the terminal/journal by the caller.
pub fn desktop_notify(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body.replace('"', "\\\""),
            title.replace('"', "\\\"")
        );
        let _ = Command::new("osascript").args(["-e", &script]).status();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = Command::new("notify-send")
            .args(["-u", "critical", title, body])
            .status();
    }
}
