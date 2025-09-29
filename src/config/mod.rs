//! Configuration management module
//! 
//! Handles loading configuration from multiple sources with proper precedence:
//! CLI arguments > environment variables > TOML files > defaults

use crate::error::ConfigError;

pub mod toml;
pub mod env;
pub mod cli;

/// Main configuration structure
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub warp_container_pattern: String,
    pub target_container_label: String,
    pub network_preference_label: String,
    pub routing_rules: Vec<RoutingRule>,
    pub log_level: String,
    pub docker_socket: String,
}

/// Routing rule configuration
#[derive(Debug, Clone)]
pub struct RoutingRule {
    pub destination: String, // CIDR notation
    pub protocol: Option<String>,
    pub port_range: Option<(u16, u16)>,
}

/// Configuration manager trait
pub trait ConfigurationManager {
    fn load_configuration(&self) -> Result<AppConfig, ConfigError>;
    fn get_warp_container_pattern(&self) -> &str;
    fn get_target_container_label(&self) -> &str;
    fn get_network_preference_label(&self) -> &str;
    fn get_routing_rules(&self) -> &[RoutingRule];
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            warp_container_pattern: "warp-*".to_string(),
            target_container_label: "network.warp.target".to_string(),
            network_preference_label: "network.warp.network".to_string(),
            routing_rules: vec![
                RoutingRule {
                    destination: "0.0.0.0/0".to_string(),
                    protocol: None,
                    port_range: None,
                }
            ],
            log_level: "info".to_string(),
            docker_socket: "/var/run/docker.sock".to_string(),
        }
    }
}