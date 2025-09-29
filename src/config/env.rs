//! Environment variable configuration handling

use std::env;
use crate::config::{AppConfig, RoutingRule};

/// Environment variable prefix
const ENV_PREFIX: &str = "DOCKER_NETWORK_WARP_";

/// Load configuration from environment variables
pub fn load_env_config() -> AppConfig {
    let mut config = AppConfig::default();
    
    if let Ok(pattern) = env::var(format!("{}WARP_CONTAINER_PATTERN", ENV_PREFIX)) {
        config.warp_container_pattern = pattern;
    }
    
    if let Ok(label) = env::var(format!("{}TARGET_CONTAINER_LABEL", ENV_PREFIX)) {
        config.target_container_label = label;
    }
    
    if let Ok(label) = env::var(format!("{}NETWORK_PREFERENCE_LABEL", ENV_PREFIX)) {
        config.network_preference_label = label;
    }
    
    if let Ok(level) = env::var(format!("{}LOG_LEVEL", ENV_PREFIX)) {
        config.log_level = level;
    }
    
    if let Ok(socket) = env::var(format!("{}DOCKER_SOCKET", ENV_PREFIX)) {
        config.docker_socket = socket;
    }
    
    // TODO: Parse routing rules from environment variables
    
    config
}