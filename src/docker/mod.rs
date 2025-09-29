//! Docker API integration module
//! 
//! Handles Docker API connections, event monitoring, and container classification

use std::collections::HashMap;
use crate::error::{EventError, HandlerError};

pub mod events;
pub mod classifier;

/// Container information structure
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub labels: HashMap<String, String>,
    pub networks: Vec<NetworkInfo>,
    pub state: ContainerState,
}

/// Network information for containers
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub name: String,
    pub ip_address: String,
    pub gateway: Option<String>,
    pub subnet: String,
}

/// Container state enumeration
#[derive(Debug, Clone)]
pub enum ContainerState {
    Starting,
    Running,
    Stopping,
    Stopped,
}

/// Container start event
#[derive(Debug, Clone)]
pub struct ContainerStartEvent {
    pub container: ContainerInfo,
}

/// Event monitor trait
pub trait EventMonitor {
    async fn start_monitoring(&self) -> Result<(), EventError>;
    async fn stop_monitoring(&self) -> Result<(), EventError>;
    fn subscribe_to_events(&self, handler: Box<dyn EventHandler>) -> Result<(), EventError>;
}

/// Event handler trait
pub trait EventHandler: Send + Sync {
    fn handle_container_start(&self, event: ContainerStartEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send + '_>>;
}