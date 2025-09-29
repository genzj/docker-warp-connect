//! Network operations module
//! 
//! Handles network namespace operations and container network discovery

use std::net::IpAddr;
use crate::error::NetworkError;

pub mod namespace;
pub mod discovery;

/// Network namespace representation
#[derive(Debug, Clone)]
pub struct NetworkNamespace {
    pub path: String,
    pub container_id: String,
}

/// Network manager trait
pub trait NetworkManager {
    async fn get_container_namespace(&self, container_id: &str) -> Result<NetworkNamespace, NetworkError>;
    async fn get_container_networks(&self, container_id: &str) -> Result<Vec<crate::docker::NetworkInfo>, NetworkError>;
    async fn resolve_container_ip(&self, container_id: &str, network: Option<&str>) -> Result<IpAddr, NetworkError>;
}