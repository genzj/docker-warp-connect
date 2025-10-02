//! Configuration management module
//!
//! Handles loading configuration from multiple sources with proper precedence:
//! CLI arguments > environment variables > TOML files > defaults

use crate::error::ConfigError;
use serde::{Deserialize, Serialize};

pub mod cli;
pub mod env;
pub mod toml;

// Default configuration constants
pub const DEFAULT_WARP_CONTAINER_PATTERN: &str = "warp-*";
pub const DEFAULT_TARGET_CONTAINER_LABEL: &str = "network.warp.target";
pub const DEFAULT_NETWORK_PREFERENCE_LABEL: &str = "network.warp.network";
pub const DEFAULT_LOG_LEVEL: &str = "info";
pub const DEFAULT_DOCKER_SOCKET: &str = "/var/run/docker.sock";
pub const DEFAULT_DOCKER_CONNECTION_METHOD: &str = "socket";

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub warp_container_pattern: String,
    pub target_container_label: String,
    pub network_preference_label: String,
    pub routing_rules: Vec<RoutingRule>,
    pub log_level: String,
    pub docker_socket: String,
    pub docker_connection_method: String,
}

/// Routing rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub destination: String, // CIDR notation
    pub protocol: Option<String>,
    pub port_range: Option<(u16, u16)>,
}

/// Configuration manager trait
pub trait ConfigurationManager {
    /// Load configuration from all sources with proper precedence
    fn load_configuration(&self) -> Result<AppConfig, ConfigError>;

    /// Get the warp container name pattern
    fn get_warp_container_pattern(&self) -> &str;

    /// Get the target container label name
    fn get_target_container_label(&self) -> &str;

    /// Get the network preference label name
    fn get_network_preference_label(&self) -> &str;

    /// Get the routing rules
    fn get_routing_rules(&self) -> &[RoutingRule];

    /// Validate configuration values
    fn validate_configuration(&self, config: &AppConfig) -> Result<(), ConfigError>;
}

/// Default configuration implementation
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            warp_container_pattern: DEFAULT_WARP_CONTAINER_PATTERN.to_string(),
            target_container_label: DEFAULT_TARGET_CONTAINER_LABEL.to_string(),
            network_preference_label: DEFAULT_NETWORK_PREFERENCE_LABEL.to_string(),
            routing_rules: vec![RoutingRule {
                destination: "0.0.0.0/0".to_string(),
                protocol: None,
                port_range: None,
            }],
            log_level: DEFAULT_LOG_LEVEL.to_string(),
            docker_socket: DEFAULT_DOCKER_SOCKET.to_string(),
            docker_connection_method: DEFAULT_DOCKER_CONNECTION_METHOD.to_string(),
        }
    }
}

impl AppConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate warp container pattern is not empty
        if self.warp_container_pattern.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "Warp container pattern cannot be empty".to_string(),
            ));
        }

        // Validate target container label is not empty
        if self.target_container_label.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "Target container label cannot be empty".to_string(),
            ));
        }

        // Validate network preference label is not empty
        if self.network_preference_label.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "Network preference label cannot be empty".to_string(),
            ));
        }

        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
                    self.log_level
                )))
            }
        }

        // Validate docker connection method
        match self.docker_connection_method.to_lowercase().as_str() {
            "socket" | "http" | "ssl" => {}
            _ => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid docker connection method: {}. Must be one of: socket, http, ssl",
                    self.docker_connection_method
                )))
            }
        }

        // Validate routing rules
        for (i, rule) in self.routing_rules.iter().enumerate() {
            if rule.destination.trim().is_empty() {
                return Err(ConfigError::ValidationError(format!(
                    "Routing rule {} has empty destination",
                    i
                )));
            }

            // Basic CIDR validation - check if it contains '/' and has valid format
            if !rule.destination.contains('/') {
                return Err(ConfigError::ValidationError(format!(
                    "Routing rule {} destination '{}' is not in CIDR format",
                    i, rule.destination
                )));
            }

            // Validate port range if specified
            if let Some((start, end)) = rule.port_range {
                if start > end {
                    return Err(ConfigError::ValidationError(format!(
                        "Routing rule {} has invalid port range: {} > {}",
                        i, start, end
                    )));
                }
            }
        }

        Ok(())
    }
}
/// Default configuration manager implementation
pub struct DefaultConfigurationManager {
    config: AppConfig,
}

impl DefaultConfigurationManager {
    /// Create a new configuration manager with CLI arguments
    pub fn new(cli_args: &cli::CliArgs) -> Result<Self, ConfigError> {
        // Start with default configuration
        let mut config = AppConfig::default();

        // Apply TOML configuration if specified
        if let Some(ref config_path) = cli_args.config {
            if let Some(toml_config) = toml::load_toml_config_optional(config_path)? {
                config = toml_config.to_app_config(config);
            }
        }

        // Apply environment variables
        config = env::apply_env_config(config)?;

        // Apply CLI arguments (highest precedence)
        config = cli_args.apply_to_config(config)?;

        // Validate final configuration
        config.validate()?;

        Ok(Self { config })
    }

    /// Create a configuration manager from a specific config file
    pub fn from_file<P: AsRef<std::path::Path>>(config_path: P) -> Result<Self, ConfigError> {
        let mut config = AppConfig::default();

        // Load TOML configuration
        if let Some(toml_config) = toml::load_toml_config_optional(config_path)? {
            config = toml_config.to_app_config(config);
        }

        // Apply environment variables
        config = env::apply_env_config(config)?;

        // Validate final configuration
        config.validate()?;

        Ok(Self { config })
    }

    /// Create a configuration manager with defaults only (for testing)
    pub fn default() -> Result<Self, ConfigError> {
        let config = AppConfig::default();
        config.validate()?;
        Ok(Self { config })
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }
}

impl ConfigurationManager for DefaultConfigurationManager {
    fn load_configuration(&self) -> Result<AppConfig, ConfigError> {
        Ok(self.config.clone())
    }

    fn get_warp_container_pattern(&self) -> &str {
        &self.config.warp_container_pattern
    }

    fn get_target_container_label(&self) -> &str {
        &self.config.target_container_label
    }

    fn get_network_preference_label(&self) -> &str {
        &self.config.network_preference_label
    }

    fn get_routing_rules(&self) -> &[RoutingRule] {
        &self.config.routing_rules
    }

    fn validate_configuration(&self, config: &AppConfig) -> Result<(), ConfigError> {
        config.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use defer;
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_app_config_validation_valid() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_app_config_validation_empty_pattern() {
        let mut config = AppConfig::default();
        config.warp_container_pattern = "".to_string();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_app_config_validation_invalid_log_level() {
        let mut config = AppConfig::default();
        config.log_level = "invalid".to_string();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_app_config_validation_invalid_docker_method() {
        let mut config = AppConfig::default();
        config.docker_connection_method = "invalid".to_string();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_app_config_validation_invalid_cidr() {
        let mut config = AppConfig::default();
        config.routing_rules = vec![RoutingRule {
            destination: "10.0.0.0".to_string(), // Missing /mask
            protocol: None,
            port_range: None,
        }];
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_app_config_validation_invalid_port_range() {
        let mut config = AppConfig::default();
        config.routing_rules = vec![RoutingRule {
            destination: "10.0.0.0/8".to_string(),
            protocol: Some("tcp".to_string()),
            port_range: Some((443, 80)), // start > end
        }];
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ValidationError(_))
        ));
    }

    #[test]
    fn test_default_configuration_manager() {
        let manager = DefaultConfigurationManager::default().unwrap();
        let config = manager.load_configuration().unwrap();

        assert_eq!(
            config.warp_container_pattern,
            DEFAULT_WARP_CONTAINER_PATTERN
        );
        assert_eq!(
            config.target_container_label,
            DEFAULT_TARGET_CONTAINER_LABEL
        );
        assert_eq!(
            config.network_preference_label,
            DEFAULT_NETWORK_PREFERENCE_LABEL
        );
        assert_eq!(config.log_level, DEFAULT_LOG_LEVEL);
        assert_eq!(config.docker_socket, DEFAULT_DOCKER_SOCKET);
        assert_eq!(
            config.docker_connection_method,
            DEFAULT_DOCKER_CONNECTION_METHOD
        );
    }

    #[test]
    fn test_configuration_manager_with_toml() {
        cleanup_env_vars();

        let toml_content = r#"
warp_container_name_pattern = "custom-*"
target_container_label = "custom.label"

[logging]
level = "debug"

[docker]
socket = "/custom/docker.sock"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();

        let manager = DefaultConfigurationManager::from_file(temp_file.path()).unwrap();
        let config = manager.load_configuration().unwrap();
        assert_eq!(config.warp_container_pattern, "custom-*");
        assert_eq!(config.target_container_label, "custom.label");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.docker_socket, "/custom/docker.sock");
    }

    // Clean up any existing env vars first
    fn cleanup_env_vars() {
        println!("mod.rs: clean up env vars");
        env::remove_var("DOCKER_NETWORK_WARP_LOG_LEVEL");
        env::remove_var("DOCKER_NETWORK_WARP_WARP_CONTAINER_PATTERN");
    }

    #[test]
    fn test_configuration_manager_precedence() {
        println!("mod.rs: in config precedence test");

        // Set up environment variables
        env::set_var("DOCKER_NETWORK_WARP_LOG_LEVEL", "trace");
        env::set_var("DOCKER_NETWORK_WARP_WARP_CONTAINER_PATTERN", "env-*");

        // Ensure cleanup happens even if test fails
        defer::defer! { cleanup_env_vars() };

        let toml_content = r#"
warp_container_name_pattern = "toml-*"
target_container_label = "toml.label"

[logging]
level = "warn"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();

        let cli_args = cli::CliArgs {
            config: Some(temp_file.path().to_string_lossy().to_string()),
            log_level: None, // Should use env var (trace)
            docker_connection_method: None,
            docker_socket: None,
            warp_container_pattern: Some("cli-*".to_string()), // Should override env and toml
            target_container_label: None,                      // Should use toml value
            network_preference_label: None,
            routing_rules: None,
            validate_config: false,
            print_default_config: false,
        };

        let manager = DefaultConfigurationManager::new(&cli_args).unwrap();
        let config = manager.load_configuration().unwrap();

        // CLI should override env and toml
        assert_eq!(config.warp_container_pattern, "cli-*");

        // Env should override toml
        assert_eq!(config.log_level, "trace");

        // TOML should be used when no env or cli override
        assert_eq!(config.target_container_label, "toml.label");
    }
}
