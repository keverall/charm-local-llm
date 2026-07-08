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

pub fn resolve_project_root(override_path: Option<PathBuf>) -> PathBuf {
    if let Some(p) = override_path {
        let canonical = std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from("."));
        return canonical;
    }

    if let Ok(current) = std::env::current_dir() {
        if current.join("Cargo.toml").exists() {
            return current;
        }
    }

    if let Ok(current) = std::env::current_dir() {
        if current.file_name().map(|n| n == "scripts").unwrap_or(false) {
            return current.parent().unwrap_or(&current).to_path_buf();
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
