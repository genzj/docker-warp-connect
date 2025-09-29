use clap::Parser;
use tracing::info;
use tracing_subscriber;

mod config;
mod docker;
mod network;
mod routing;
mod error;

use crate::config::cli::CliArgs;
use crate::error::AppError;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = CliArgs::parse();
    
    // Initialize logging
    let log_level = cli.log_level.as_deref().unwrap_or("info");
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();
    
    info!("Starting Docker Network Warp");
    
    // TODO: Initialize configuration manager
    // TODO: Initialize Docker event monitor
    // TODO: Initialize network manager
    // TODO: Initialize route manager
    // TODO: Start main event processing loop
    
    info!("Docker Network Warp started successfully");
    
    // Keep the application running
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down Docker Network Warp");
    Ok(())
}