//! TOML configuration file parsing

use crate::config::{AppConfig, RoutingRule};
use crate::error::ConfigError;
use serde::Deserialize;
use std::path::Path;

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

impl TomlConfig {
    /// Convert TomlConfig to AppConfig, applying values over defaults
    pub fn to_app_config(&self, base_config: AppConfig) -> AppConfig {
        let mut config = base_config;

        if let Some(ref method) = self.docker_connection_method {
            config.docker_connection_method = method.clone();
        }

        if let Some(ref pattern) = self.warp_container_name_pattern {
            config.warp_container_pattern = pattern.clone();
        }

        if let Some(ref label) = self.target_container_label {
            config.target_container_label = label.clone();
        }

        if let Some(ref label) = self.network_preference_label {
            config.network_preference_label = label.clone();
        }

        if let Some(ref rules) = self.routing_rules {
            config.routing_rules = rules
                .iter()
                .map(|r| RoutingRule {
                    destination: r.destination.clone(),
                    protocol: r.protocol.clone(),
                    port_range: r.port_range,
                })
                .collect();
        }

        if let Some(ref logging) = self.logging {
            if let Some(ref level) = logging.level {
                config.log_level = level.clone();
            }
        }

        if let Some(ref docker) = self.docker {
            if let Some(ref socket) = docker.socket {
                config.docker_socket = socket.clone();
            }
        }

        config
    }
}

/// Load configuration from TOML file
pub fn load_toml_config<P: AsRef<Path>>(path: P) -> Result<TomlConfig, ConfigError> {
    let path_str = path.as_ref().to_string_lossy().to_string();

    // Check if file exists
    if !path.as_ref().exists() {
        return Err(ConfigError::FileNotFound { path: path_str });
    }

    let content = std::fs::read_to_string(&path).map_err(|e| {
        ConfigError::InvalidFormat(format!("Failed to read file {}: {}", path_str, e))
    })?;

    toml::from_str(&content).map_err(|e| {
        ConfigError::InvalidFormat(format!("Failed to parse TOML from {}: {}", path_str, e))
    })
}

/// Load configuration from TOML file if it exists, otherwise return None
pub fn load_toml_config_optional<P: AsRef<Path>>(
    path: P,
) -> Result<Option<TomlConfig>, ConfigError> {
    if !path.as_ref().exists() {
        return Ok(None);
    }

    load_toml_config(path).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_toml_config() {
        let toml_content = r#"
docker_connection_method = "socket"
warp_container_name_pattern = "proxy-*"
target_container_label = "app.proxy.target"
network_preference_label = "app.proxy.network"

[logging]
level = "debug"
format = "json"

[docker]
socket = "/var/run/docker.sock"
api_version = "1.41"

[[routing_rules]]
destination = "10.0.0.0/8"
protocol = "tcp"
port_range = [80, 443]

[[routing_rules]]
destination = "192.168.0.0/16"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();

        let config = load_toml_config(temp_file.path()).unwrap();

        assert_eq!(config.docker_connection_method, Some("socket".to_string()));
        assert_eq!(
            config.warp_container_name_pattern,
            Some("proxy-*".to_string())
        );
        assert_eq!(
            config.target_container_label,
            Some("app.proxy.target".to_string())
        );
        assert_eq!(
            config.network_preference_label,
            Some("app.proxy.network".to_string())
        );

        let logging = config.logging.unwrap();
        assert_eq!(logging.level, Some("debug".to_string()));
        assert_eq!(logging.format, Some("json".to_string()));

        let docker = config.docker.unwrap();
        assert_eq!(docker.socket, Some("/var/run/docker.sock".to_string()));
        assert_eq!(docker.api_version, Some("1.41".to_string()));

        let rules = config.routing_rules.unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].destination, "10.0.0.0/8");
        assert_eq!(rules[0].protocol, Some("tcp".to_string()));
        assert_eq!(rules[0].port_range, Some((80, 443)));
        assert_eq!(rules[1].destination, "192.168.0.0/16");
        assert_eq!(rules[1].protocol, None);
        assert_eq!(rules[1].port_range, None);
    }

    #[test]
    fn test_load_minimal_toml_config() {
        let toml_content = r#"
warp_container_name_pattern = "warp-*"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();

        let config = load_toml_config(temp_file.path()).unwrap();

        assert_eq!(
            config.warp_container_name_pattern,
            Some("warp-*".to_string())
        );
        assert_eq!(config.docker_connection_method, None);
        assert_eq!(config.target_container_label, None);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_toml_config("/nonexistent/file.toml");
        assert!(matches!(result, Err(ConfigError::FileNotFound { .. })));
    }

    #[test]
    fn test_load_invalid_toml() {
        let invalid_toml = r#"
invalid toml content [[[
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_toml.as_bytes()).unwrap();

        let result = load_toml_config(temp_file.path());
        assert!(matches!(result, Err(ConfigError::InvalidFormat(_))));
    }

    #[test]
    fn test_toml_config_to_app_config() {
        let toml_config = TomlConfig {
            docker_connection_method: Some("http".to_string()),
            warp_container_name_pattern: Some("custom-*".to_string()),
            target_container_label: Some("custom.label".to_string()),
            network_preference_label: Some("custom.network".to_string()),
            routing_rules: Some(vec![TomlRoutingRule {
                destination: "172.16.0.0/12".to_string(),
                protocol: Some("udp".to_string()),
                port_range: Some((53, 53)),
            }]),
            logging: Some(LoggingConfig {
                level: Some("trace".to_string()),
                format: Some("plain".to_string()),
            }),
            docker: Some(DockerConfig {
                socket: Some("/custom/docker.sock".to_string()),
                api_version: Some("1.40".to_string()),
            }),
        };

        let base_config = AppConfig::default();
        let app_config = toml_config.to_app_config(base_config);

        assert_eq!(app_config.docker_connection_method, "http");
        assert_eq!(app_config.warp_container_pattern, "custom-*");
        assert_eq!(app_config.target_container_label, "custom.label");
        assert_eq!(app_config.network_preference_label, "custom.network");
        assert_eq!(app_config.log_level, "trace");
        assert_eq!(app_config.docker_socket, "/custom/docker.sock");
        assert_eq!(app_config.routing_rules.len(), 1);
        assert_eq!(app_config.routing_rules[0].destination, "172.16.0.0/12");
        assert_eq!(
            app_config.routing_rules[0].protocol,
            Some("udp".to_string())
        );
        assert_eq!(app_config.routing_rules[0].port_range, Some((53, 53)));
    }
}
