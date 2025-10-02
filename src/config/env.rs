//! Environment variable configuration handling

use crate::config::{AppConfig, RoutingRule};
use crate::error::ConfigError;
use std::env;

/// Environment variable prefix
const ENV_PREFIX: &str = "DOCKER_NETWORK_WARP_";

/// Apply environment variable configuration over base configuration
pub fn apply_env_config(mut base_config: AppConfig) -> Result<AppConfig, ConfigError> {
    // Apply individual configuration values
    if let Ok(method) = env::var(format!("{}DOCKER_CONNECTION_METHOD", ENV_PREFIX)) {
        base_config.docker_connection_method = method;
    }

    if let Ok(pattern) = env::var(format!("{}WARP_CONTAINER_PATTERN", ENV_PREFIX)) {
        base_config.warp_container_pattern = pattern;
    }

    if let Ok(label) = env::var(format!("{}TARGET_CONTAINER_LABEL", ENV_PREFIX)) {
        base_config.target_container_label = label;
    }

    if let Ok(label) = env::var(format!("{}NETWORK_PREFERENCE_LABEL", ENV_PREFIX)) {
        base_config.network_preference_label = label;
    }

    if let Ok(level) = env::var(format!("{}LOG_LEVEL", ENV_PREFIX)) {
        base_config.log_level = level;
    }

    if let Ok(socket) = env::var(format!("{}DOCKER_SOCKET", ENV_PREFIX)) {
        base_config.docker_socket = socket;
    }

    // Parse routing rules from environment variables
    // Format: DOCKER_NETWORK_WARP_ROUTING_RULES="dest1:proto1:port1-port2,dest2:proto2:port3-port4"
    if let Ok(rules_str) = env::var(format!("{}ROUTING_RULES", ENV_PREFIX)) {
        base_config.routing_rules = parse_routing_rules_from_env(&rules_str)?;
    }

    Ok(base_config)
}

/// Parse routing rules from environment variable string
/// Format: "dest1:proto1:port1-port2,dest2:proto2:port3-port4"
/// Examples:
/// - "0.0.0.0/0" (destination only)
/// - "10.0.0.0/8:tcp" (destination and protocol)
/// - "192.168.0.0/16:tcp:80-443" (destination, protocol, and port range)
/// - "172.16.0.0/12::53-53" (destination and port range, no protocol)
pub fn parse_routing_rules_from_env(rules_str: &str) -> Result<Vec<RoutingRule>, ConfigError> {
    if rules_str.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut rules = Vec::new();

    for rule_str in rules_str.split(',') {
        let rule_str = rule_str.trim();
        if rule_str.is_empty() {
            continue;
        }

        let parts: Vec<&str> = rule_str.split(':').collect();

        if parts.is_empty() || parts[0].trim().is_empty() {
            return Err(ConfigError::InvalidFormat(
                format!("Invalid routing rule format: '{}'. Expected format: 'destination[:protocol[:port_start-port_end]]'", rule_str)
            ));
        }

        let destination = parts[0].trim().to_string();

        // Validate CIDR format
        if !destination.contains('/') {
            return Err(ConfigError::InvalidFormat(
                format!("Invalid destination '{}' in routing rule. Must be in CIDR format (e.g., '10.0.0.0/8')", destination)
            ));
        }

        let protocol = if parts.len() > 1 && !parts[1].trim().is_empty() {
            Some(parts[1].trim().to_string())
        } else {
            None
        };

        let port_range = if parts.len() > 2 && !parts[2].trim().is_empty() {
            Some(parse_port_range(parts[2].trim())?)
        } else {
            None
        };

        rules.push(RoutingRule {
            destination,
            protocol,
            port_range,
        });
    }

    Ok(rules)
}

/// Parse port range from string format "start-end" or "port"
fn parse_port_range(port_str: &str) -> Result<(u16, u16), ConfigError> {
    if port_str.contains('-') {
        let parts: Vec<&str> = port_str.split('-').collect();
        if parts.len() != 2 {
            return Err(ConfigError::InvalidFormat(format!(
                "Invalid port range format: '{}'. Expected 'start-end' or single port",
                port_str
            )));
        }

        let start = parts[0].trim().parse::<u16>().map_err(|_| {
            ConfigError::InvalidFormat(format!("Invalid start port: '{}'", parts[0]))
        })?;

        let end = parts[1]
            .trim()
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidFormat(format!("Invalid end port: '{}'", parts[1])))?;

        if start > end {
            return Err(ConfigError::InvalidFormat(format!(
                "Invalid port range: start port {} is greater than end port {}",
                start, end
            )));
        }

        Ok((start, end))
    } else {
        let port = port_str
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidFormat(format!("Invalid port: '{}'", port_str)))?;
        Ok((port, port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_env_vars() {
        env::set_var("DOCKER_NETWORK_WARP_DOCKER_CONNECTION_METHOD", "http");
        env::set_var("DOCKER_NETWORK_WARP_WARP_CONTAINER_PATTERN", "proxy-*");
        env::set_var(
            "DOCKER_NETWORK_WARP_TARGET_CONTAINER_LABEL",
            "app.proxy.target",
        );
        env::set_var(
            "DOCKER_NETWORK_WARP_NETWORK_PREFERENCE_LABEL",
            "app.proxy.network",
        );
        env::set_var("DOCKER_NETWORK_WARP_LOG_LEVEL", "debug");
        env::set_var("DOCKER_NETWORK_WARP_DOCKER_SOCKET", "/custom/docker.sock");
        env::set_var(
            "DOCKER_NETWORK_WARP_ROUTING_RULES",
            "10.0.0.0/8:tcp:80-443,192.168.0.0/16::53-53,172.16.0.0/12",
        );
    }

    fn cleanup_env_vars() {
        env::remove_var("DOCKER_NETWORK_WARP_DOCKER_CONNECTION_METHOD");
        env::remove_var("DOCKER_NETWORK_WARP_WARP_CONTAINER_PATTERN");
        env::remove_var("DOCKER_NETWORK_WARP_TARGET_CONTAINER_LABEL");
        env::remove_var("DOCKER_NETWORK_WARP_NETWORK_PREFERENCE_LABEL");
        env::remove_var("DOCKER_NETWORK_WARP_LOG_LEVEL");
        env::remove_var("DOCKER_NETWORK_WARP_DOCKER_SOCKET");
        env::remove_var("DOCKER_NETWORK_WARP_ROUTING_RULES");
    }

    #[test]
    fn test_apply_env_config() {
        // Clean up first to ensure no interference
        cleanup_env_vars();
        setup_env_vars();

        let base_config = AppConfig::default();
        let config = apply_env_config(base_config).unwrap();

        assert_eq!(config.docker_connection_method, "http");
        assert_eq!(config.warp_container_pattern, "proxy-*");
        assert_eq!(config.target_container_label, "app.proxy.target");
        assert_eq!(config.network_preference_label, "app.proxy.network");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.docker_socket, "/custom/docker.sock");

        assert_eq!(config.routing_rules.len(), 3);

        // First rule: 10.0.0.0/8:tcp:80-443
        assert_eq!(config.routing_rules[0].destination, "10.0.0.0/8");
        assert_eq!(config.routing_rules[0].protocol, Some("tcp".to_string()));
        assert_eq!(config.routing_rules[0].port_range, Some((80, 443)));

        // Second rule: 192.168.0.0/16::53-53
        assert_eq!(config.routing_rules[1].destination, "192.168.0.0/16");
        assert_eq!(config.routing_rules[1].protocol, None);
        assert_eq!(config.routing_rules[1].port_range, Some((53, 53)));

        // Third rule: 172.16.0.0/12
        assert_eq!(config.routing_rules[2].destination, "172.16.0.0/12");
        assert_eq!(config.routing_rules[2].protocol, None);
        assert_eq!(config.routing_rules[2].port_range, None);

        cleanup_env_vars();
    }

    #[test]
    fn test_apply_env_config_no_vars() {
        cleanup_env_vars();

        let base_config = AppConfig::default();
        let config = apply_env_config(base_config.clone()).unwrap();

        // Should be unchanged from base config
        assert_eq!(
            config.docker_connection_method,
            base_config.docker_connection_method
        );
        assert_eq!(
            config.warp_container_pattern,
            base_config.warp_container_pattern
        );
        assert_eq!(
            config.target_container_label,
            base_config.target_container_label
        );
        assert_eq!(
            config.network_preference_label,
            base_config.network_preference_label
        );
        assert_eq!(config.log_level, base_config.log_level);
        assert_eq!(config.docker_socket, base_config.docker_socket);
        assert_eq!(config.routing_rules.len(), base_config.routing_rules.len());
    }

    #[test]
    fn test_parse_routing_rules_from_env() {
        let rules_str = "10.0.0.0/8:tcp:80-443,192.168.0.0/16::53-53,172.16.0.0/12";
        let rules = parse_routing_rules_from_env(rules_str).unwrap();

        assert_eq!(rules.len(), 3);

        assert_eq!(rules[0].destination, "10.0.0.0/8");
        assert_eq!(rules[0].protocol, Some("tcp".to_string()));
        assert_eq!(rules[0].port_range, Some((80, 443)));

        assert_eq!(rules[1].destination, "192.168.0.0/16");
        assert_eq!(rules[1].protocol, None);
        assert_eq!(rules[1].port_range, Some((53, 53)));

        assert_eq!(rules[2].destination, "172.16.0.0/12");
        assert_eq!(rules[2].protocol, None);
        assert_eq!(rules[2].port_range, None);
    }

    #[test]
    fn test_parse_routing_rules_empty() {
        let rules = parse_routing_rules_from_env("").unwrap();
        assert_eq!(rules.len(), 0);

        let rules = parse_routing_rules_from_env("   ").unwrap();
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_parse_routing_rules_invalid_cidr() {
        let result = parse_routing_rules_from_env("10.0.0.0:tcp:80-443");
        assert!(matches!(result, Err(ConfigError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_routing_rules_invalid_port_range() {
        let result = parse_routing_rules_from_env("10.0.0.0/8:tcp:443-80");
        assert!(matches!(result, Err(ConfigError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_port_range() {
        assert_eq!(parse_port_range("80").unwrap(), (80, 80));
        assert_eq!(parse_port_range("80-443").unwrap(), (80, 443));
        assert_eq!(parse_port_range("53-53").unwrap(), (53, 53));

        assert!(parse_port_range("443-80").is_err());
        assert!(parse_port_range("invalid").is_err());
        assert!(parse_port_range("80-443-8080").is_err());
    }
}
