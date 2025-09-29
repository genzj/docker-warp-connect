//! Routing table management module
//! 
//! Handles routing table operations within container network namespaces

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use ipnetwork::IpNetwork as ExternalIpNetwork;
use crate::network::NetworkNamespace;
use crate::error::RouteError;

pub mod manager;
pub mod rules;

/// Route entry structure
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub destination: IpNetwork,
    pub gateway: IpAddr,
    pub interface: Option<String>,
    pub metric: Option<u32>,
}

/// IP network representation
#[derive(Debug, Clone)]
pub enum IpNetwork {
    V4 { addr: Ipv4Addr, prefix: u8 },
    V6 { addr: Ipv6Addr, prefix: u8 },
}

/// Route manager trait
pub trait RouteManager {
    async fn add_route(&self, namespace: &NetworkNamespace, route: &RouteEntry) -> Result<(), RouteError>;
    async fn remove_route(&self, namespace: &NetworkNamespace, route: &RouteEntry) -> Result<(), RouteError>;
    async fn list_routes(&self, namespace: &NetworkNamespace) -> Result<Vec<RouteEntry>, RouteError>;
}