//! Command-line argument parsing

use crate::config::{AppConfig, RoutingRule};
use crate::error::ConfigError;
use clap::Parser;

/// Command-line arguments structure
#[derive(Parser, Debug)]
#[command(name = "docker-network-warp")]
#[command(about = "Automatic Docker container network routing manager")]
#[command(version)]
pub struct CliArgs {
    /// Configuration file path
    #[arg(short, long, help = "Path to TOML configuration file")]
    pub config: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, help = "Set the logging level")]
    pub log_level: Option<String>,

    /// Docker connection method (socket, http, ssl)
    #[arg(long, help = "Docker connection method")]
    pub docker_connection_method: Option<String>,

    /// Docker socket path
    #[arg(long, help = "Path to Docker socket")]
    pub docker_socket: Option<String>,

    /// Warp container name pattern
    #[arg(
        long,
        help = "Pattern to match warp container names (supports wildcards)"
    )]
    pub warp_container_pattern: Option<String>,

    /// Target container label name
    #[arg(long, help = "Label name used to identify target containers")]
    pub target_container_label: Option<String>,

    /// Network preference label name
    #[arg(
        long,
        help = "Label name used to specify network preference for warp containers"
    )]
    pub network_preference_label: Option<String>,

    /// Routing rules in format "dest:proto:port_range"
    #[arg(
        long,
        help = "Routing rules in format 'dest1:proto1:port1-port2,dest2:proto2:port3-port4'"
    )]
    pub routing_rules: Option<String>,

    /// Validate configuration and exit
    #[arg(
        long,
        help = "Validate configuration and exit without starting the service"
    )]
    pub validate_config: bool,

    /// Print default configuration and exit
    #[arg(long, help = "Print default configuration in TOML format and exit")]
    pub print_default_config: bool,
}

impl CliArgs {
    /// Apply CLI arguments over base configuration
    pub fn apply_to_config(&self, mut base_config: AppConfig) -> Result<AppConfig, ConfigError> {
        if let Some(ref method) = self.docker_connection_method {
            base_config.docker_connection_method = method.clone();
        }

        if let Some(ref pattern) = self.warp_container_pattern {
            base_config.warp_container_pattern = pattern.clone();
        }

        if let Some(ref label) = self.target_container_label {
            base_config.target_container_label = label.clone();
        }

        if let Some(ref label) = self.network_preference_label {
            base_config.network_preference_label = label.clone();
        }

        if let Some(ref level) = self.log_level {
            base_config.log_level = level.clone();
        }

        if let Some(ref socket) = self.docker_socket {
            base_config.docker_socket = socket.clone();
        }

        if let Some(ref rules_str) = self.routing_rules {
            base_config.routing_rules = parse_routing_rules_from_cli(rules_str)?;
        }

        Ok(base_config)
    }
}

/// Parse routing rules from CLI argument string
/// Same format as environment variables: "dest1:proto1:port1-port2,dest2:proto2:port3-port4"
fn parse_routing_rules_from_cli(rules_str: &str) -> Result<Vec<RoutingRule>, ConfigError> {
    // Reuse the same parsing logic as environment variables
    crate::config::env::parse_routing_rules_from_env(rules_str)
}

/// Print default configuration in TOML format
pub fn print_default_config() {
    let default_config = AppConfig::default();

    println!("# Docker Network Warp Configuration");
    println!("# This is the default configuration with all available options");
    println!();
    println!("# Docker connection method: socket, http, or ssl");
    println!(
        "docker_connection_method = \"{}\"",
        default_config.docker_connection_method
    );
    println!();
    println!("# Pattern to match warp container names (supports wildcards)");
    println!(
        "warp_container_name_pattern = \"{}\"",
        default_config.warp_container_pattern
    );
    println!();
    println!("# Label name used to identify target containers");
    println!(
        "target_container_label = \"{}\"",
        default_config.target_container_label
    );
    println!();
    println!("# Label name used to specify network preference for warp containers");
    println!(
        "network_preference_label = \"{}\"",
        default_config.network_preference_label
    );
    println!();
    println!("[logging]");
    println!("# Log level: trace, debug, info, warn, error");
    println!("level = \"{}\"", default_config.log_level);
    println!();
    println!("[docker]");
    println!("# Path to Docker socket");
    println!("socket = \"{}\"", default_config.docker_socket);
    println!();
    println!(
        "# Routing rules - traffic matching these rules will be routed through warp containers"
    );
    for (i, rule) in default_config.routing_rules.iter().enumerate() {
        println!("[[routing_rules]]");
        println!("destination = \"{}\"", rule.destination);
        if let Some(ref protocol) = rule.protocol {
            println!("protocol = \"{}\"", protocol);
        }
        if let Some((start, end)) = rule.port_range {
            println!("port_range = [{}, {}]", start, end);
        }
        if i < default_config.routing_rules.len() - 1 {
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_args_parsing() {
        let args = CliArgs::try_parse_from([
            "docker-network-warp",
            "--config",
            "/etc/docker-network-warp.toml",
            "--log-level",
            "debug",
            "--docker-connection-method",
            "http",
            "--docker-socket",
            "/custom/docker.sock",
            "--warp-container-pattern",
            "proxy-*",
            "--target-container-label",
            "app.proxy.target",
            "--network-preference-label",
            "app.proxy.network",
            "--routing-rules",
            "10.0.0.0/8:tcp:80-443,192.168.0.0/16::53-53",
            "--validate-config",
        ])
        .unwrap();

        assert_eq!(
            args.config,
            Some("/etc/docker-network-warp.toml".to_string())
        );
        assert_eq!(args.log_level, Some("debug".to_string()));
        assert_eq!(args.docker_connection_method, Some("http".to_string()));
        assert_eq!(args.docker_socket, Some("/custom/docker.sock".to_string()));
        assert_eq!(args.warp_container_pattern, Some("proxy-*".to_string()));
        assert_eq!(
            args.target_container_label,
            Some("app.proxy.target".to_string())
        );
        assert_eq!(
            args.network_preference_label,
            Some("app.proxy.network".to_string())
        );
        assert_eq!(
            args.routing_rules,
            Some("10.0.0.0/8:tcp:80-443,192.168.0.0/16::53-53".to_string())
        );
        assert!(args.validate_config);
        assert!(!args.print_default_config);
    }

    #[test]
    fn test_cli_args_minimal() {
        let args = CliArgs::try_parse_from(["docker-network-warp"]).unwrap();

        assert_eq!(args.config, None);
        assert_eq!(args.log_level, None);
        assert_eq!(args.docker_connection_method, None);
        assert_eq!(args.docker_socket, None);
        assert_eq!(args.warp_container_pattern, None);
        assert_eq!(args.target_container_label, None);
        assert_eq!(args.network_preference_label, None);
        assert_eq!(args.routing_rules, None);
        assert!(!args.validate_config);
        assert!(!args.print_default_config);
    }

    #[test]
    fn test_apply_cli_to_config() {
        let args = CliArgs {
            config: Some("/test/config.toml".to_string()),
            log_level: Some("trace".to_string()),
            docker_connection_method: Some("ssl".to_string()),
            docker_socket: Some("/test/docker.sock".to_string()),
            warp_container_pattern: Some("test-*".to_string()),
            target_container_label: Some("test.label".to_string()),
            network_preference_label: Some("test.network".to_string()),
            routing_rules: Some("172.16.0.0/12:udp:53-53".to_string()),
            validate_config: false,
            print_default_config: false,
        };

        let base_config = AppConfig::default();
        let config = args.apply_to_config(base_config).unwrap();

        assert_eq!(config.docker_connection_method, "ssl");
        assert_eq!(config.log_level, "trace");
        assert_eq!(config.docker_socket, "/test/docker.sock");
        assert_eq!(config.warp_container_pattern, "test-*");
        assert_eq!(config.target_container_label, "test.label");
        assert_eq!(config.network_preference_label, "test.network");

        assert_eq!(config.routing_rules.len(), 1);
        assert_eq!(config.routing_rules[0].destination, "172.16.0.0/12");
        assert_eq!(config.routing_rules[0].protocol, Some("udp".to_string()));
        assert_eq!(config.routing_rules[0].port_range, Some((53, 53)));
    }

    #[test]
    fn test_apply_cli_to_config_no_overrides() {
        let args = CliArgs {
            config: None,
            log_level: None,
            docker_connection_method: None,
            docker_socket: None,
            warp_container_pattern: None,
            target_container_label: None,
            network_preference_label: None,
            routing_rules: None,
            validate_config: false,
            print_default_config: false,
        };

        let base_config = AppConfig::default();
        let original_config = base_config.clone();
        let config = args.apply_to_config(base_config).unwrap();

        // Should be unchanged
        assert_eq!(
            config.docker_connection_method,
            original_config.docker_connection_method
        );
        assert_eq!(config.log_level, original_config.log_level);
        assert_eq!(config.docker_socket, original_config.docker_socket);
        assert_eq!(
            config.warp_container_pattern,
            original_config.warp_container_pattern
        );
        assert_eq!(
            config.target_container_label,
            original_config.target_container_label
        );
        assert_eq!(
            config.network_preference_label,
            original_config.network_preference_label
        );
        assert_eq!(
            config.routing_rules.len(),
            original_config.routing_rules.len()
        );
    }
}
