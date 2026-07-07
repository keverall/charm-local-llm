pub mod cli;
pub mod commands;
pub mod config;
pub mod crush;
pub mod kilo_integration;
pub mod modelfile;
pub mod ollama;
pub mod platform;

pub use config::{Config, ConfigError, Platform};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_display_roundtrips() {
        assert_eq!(Platform::CachyOS.to_string(), "cachyos");
        assert_eq!(Platform::MacOS.to_string(), "macos");
        assert_eq!(Platform::Linux.to_string(), "linux");
    }

    #[test]
    fn config_default_has_expected_ollama_port() {
        let config = Config::default(Platform::CachyOS);
        assert_eq!(config.ollama_port, 11434);
        assert_eq!(config.qdrant_port, 6333);
    }
}
