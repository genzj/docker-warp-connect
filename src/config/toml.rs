//! TOML configuration file parsing

use serde::Deserialize;
use crate::error::ConfigError;

/// TOML configuration structure
#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    pub docker_connection_method: Option<String>,
    pub warp_container_name_pattern: Option<String>,
    pub target_container_label: Option<String>,
    pub network_preference_label: Option<String>,
    pub routing_rules: Option<Vec<TomlRoutingRule>>,
    pub logging: Option<LoggingConfig>,
    pub docker: Option<DockerConfig>,
}

/// TOML routing rule configuration
#[derive(Debug, Deserialize)]
pub struct TomlRoutingRule {
    pub destination: String,
    pub protocol: Option<String>,
    pub port_range: Option<(u16, u16)>,
}

/// Logging configuration
#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: Option<String>,
    pub format: Option<String>,
}

/// Docker configuration
#[derive(Debug, Deserialize)]
pub struct DockerConfig {
    pub socket: Option<String>,
    pub api_version: Option<String>,
}

/// Load configuration from TOML file
pub fn load_toml_config(path: &str) -> Result<TomlConfig, ConfigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| ConfigError::FileNotFound { path: path.to_string() })?;
    
    toml::from_str(&content)
        .map_err(|e| ConfigError::InvalidFormat(e.to_string()))
}