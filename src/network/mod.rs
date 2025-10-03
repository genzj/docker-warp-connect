//! Network operations module
//!
//! Handles network namespace operations and container network discovery

use crate::error::NetworkError;
use std::net::IpAddr;

pub mod discovery;
pub mod namespace;

pub use namespace::ContainerNetworkAnalysis;

/// Network namespace representation
#[derive(Debug, Clone)]
pub struct NetworkNamespace {
    pub path: String,
    pub container_id: String,
}

/// Network manager trait
pub trait NetworkManager {
    fn get_container_namespace(
        &self,
        container_id: &str,
    ) -> impl std::future::Future<Output = Result<NetworkNamespace, NetworkError>> + Send;
    fn get_container_networks(
        &self,
        container_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<crate::docker::NetworkInfo>, NetworkError>> + Send;
    fn resolve_container_ip(
        &self,
        container_id: &str,
        network: Option<&str>,
    ) -> impl std::future::Future<Output = Result<IpAddr, NetworkError>> + Send;
}
