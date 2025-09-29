//! Command-line argument parsing

use clap::Parser;

/// Command-line arguments structure
#[derive(Parser, Debug)]
#[command(name = "docker-network-warp")]
#[command(about = "Automatic Docker container network routing manager")]
pub struct CliArgs {
    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<String>,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long)]
    pub log_level: Option<String>,
    
    /// Docker socket path
    #[arg(long)]
    pub docker_socket: Option<String>,
    
    /// Warp container name pattern
    #[arg(long)]
    pub warp_container_pattern: Option<String>,
    
    /// Target container label name
    #[arg(long)]
    pub target_container_label: Option<String>,
    
    /// Network preference label name
    #[arg(long)]
    pub network_preference_label: Option<String>,
}