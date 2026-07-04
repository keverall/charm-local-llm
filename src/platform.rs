use crate::Platform;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn detect_platform(override_str: Option<&str>) -> Platform {
    if let Some(o) = override_str {
        return match o.to_lowercase().as_str() {
            "macos" | "macbook" => Platform::MacOS,
            "cachyos" => Platform::CachyOS,
            "linux" => Platform::Linux,
            _ => Platform::Unknown,
        };
    }

    if std::env::consts::OS == "macos" {
        return Platform::MacOS;
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
    match platform {
        Platform::MacOS => project_root
            .join("platform")
            .join("macbook-m4-24gb-optimized")
            .join(".env"),
        Platform::CachyOS | Platform::Linux => project_root
            .join("platform")
            .join("cachyos-i9-32gb-nvidia-4090")
            .join(".env"),
        Platform::Unknown => project_root.join(".env"),
    }
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
