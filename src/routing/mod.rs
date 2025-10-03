//! Routing table management module
//!
//! Handles routing table operations within container network namespaces

use crate::error::RouteError;
use crate::network::NetworkNamespace;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub mod manager;
pub mod rules;

/// Route entry structure
#[derive(Debug, Clone, PartialEq)]
pub struct RouteEntry {
    pub destination: IpNetwork,
    pub gateway: IpAddr,
    pub interface: Option<String>,
    pub metric: Option<u32>,
}

/// IP network representation
#[derive(Debug, Clone, PartialEq)]
pub enum IpNetwork {
    V4 { addr: Ipv4Addr, prefix: u8 },
    V6 { addr: Ipv6Addr, prefix: u8 },
}

impl IpNetwork {
    /// Create a new IPv4 network
    pub fn new_v4(addr: Ipv4Addr, prefix: u8) -> Self {
        Self::V4 { addr, prefix }
    }

    /// Create a new IPv6 network
    pub fn new_v6(addr: Ipv6Addr, prefix: u8) -> Self {
        Self::V6 { addr, prefix }
    }

    /// Get the network address as an IpAddr
    pub fn addr(&self) -> IpAddr {
        match self {
            IpNetwork::V4 { addr, .. } => IpAddr::V4(*addr),
            IpNetwork::V6 { addr, .. } => IpAddr::V6(*addr),
        }
    }

    /// Get the prefix length
    pub fn prefix(&self) -> u8 {
        match self {
            IpNetwork::V4 { prefix, .. } => *prefix,
            IpNetwork::V6 { prefix, .. } => *prefix,
        }
    }
}

/// Route manager trait
pub trait RouteManager {
    fn add_route(
        &self,
        namespace: &NetworkNamespace,
        route: &RouteEntry,
    ) -> impl std::future::Future<Output = Result<(), RouteError>> + Send;
    fn remove_route(
        &self,
        namespace: &NetworkNamespace,
        route: &RouteEntry,
    ) -> impl std::future::Future<Output = Result<(), RouteError>> + Send;
    fn list_routes(
        &self,
        namespace: &NetworkNamespace,
    ) -> impl std::future::Future<Output = Result<Vec<RouteEntry>, RouteError>> + Send;
}
