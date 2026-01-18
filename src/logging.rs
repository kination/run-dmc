//! Logging infrastructure for rundmc
//!
//! This module provides structured logging with future support for:
//! - JSON formatted logs
//! - OpenTelemetry integration
//! - Prometheus metrics
//! - Loki log aggregation

use std::path::PathBuf;

/// Log format configuration
#[derive(Debug, Clone)]
pub enum LogFormat {
    /// Human-readable text format (default)
    Text,
    /// JSON format for machine parsing (future)
    Json,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log format
    pub format: LogFormat,
    /// Log level filter (from RUST_LOG env var)
    pub level: String,
    /// Optional log file path (OCI --log parameter)
    pub file: Option<PathBuf>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Text,
            level: "info".to_string(),
            file: None,
        }
    }
}

impl LogConfig {
    /// Create configuration from environment
    pub fn from_env() -> Self {
        let level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        // Future: Check RUNDMC_LOG_FORMAT=json
        let format = match std::env::var("RUNDMC_LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            _ => LogFormat::Text,
        };

        Self {
            format,
            level,
            file: None,
        }
    }

    /// Set log file path (from OCI --log parameter)
    pub fn with_file(mut self, path: Option<PathBuf>) -> Self {
        self.file = path;
        self
    }
}

/// Initialize logging with configuration
pub fn init_with_config(config: &LogConfig) {
    match config.format {
        LogFormat::Text => init_text_logger(&config.level),
        LogFormat::Json => {
            // Future implementation
            log::warn!("JSON logging not yet implemented, falling back to text");
            init_text_logger(&config.level);
        }
    }
}

/// Initialize text-based logger
fn init_text_logger(level: &str) {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format_timestamp_millis()
        .format_module_path(true)
        .format_target(false)
        .init();
}

/// Log a container lifecycle event
///
/// Future: This will export to OpenTelemetry
#[allow(dead_code)]
pub fn log_container_event(
    event_type: &str,
    container_id: &str,
    details: &[(&str, &str)],
) {
    let mut msg = format!("Container event: type={}, id={}", event_type, container_id);
    for (key, value) in details {
        msg.push_str(&format!(", {}={}", key, value));
    }
    log::info!("{}", msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert!(matches!(config.format, LogFormat::Text));
        assert!(config.file.is_none());
    }

    #[test]
    fn test_with_file() {
        let config = LogConfig::default()
            .with_file(Some(PathBuf::from("/var/log/rundmc.log")));
        assert!(config.file.is_some());
    }
}
