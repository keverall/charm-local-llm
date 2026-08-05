use crate::Platform;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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
                let value = value.trim().trim_matches('"');
                env.insert(key.into(), value.into());
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

/// True when `dir` looks like the charm-local-llm checkout.
fn is_project_root(dir: &Path) -> bool {
    dir.join("Cargo.toml").exists() || dir.join("docker-compose.yml").exists()
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
    if is_project_root(&path) {
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
        if is_project_root(&path) {
            return path;
        }
    }

    // Walk up from the current directory looking for the checkout.
    if let Ok(current) = std::env::current_dir() {
        for dir in current.ancestors() {
            if is_project_root(dir) {
                return dir.to_path_buf();
            }
        }
    }

    // Recorded at bootstrap time — the path used by login/boot starts.
    if let Some(root) = marker_project_root() {
        return root;
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
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
