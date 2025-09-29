//! Routing rule calculation and validation

use std::net::IpAddr;
use ipnetwork::IpNetwork as ExternalIpNetwork;
use crate::routing::{RouteEntry, IpNetwork};
use crate::error::RouteError;

/// Routing rule calculator
pub struct RoutingRuleCalculator;

impl RoutingRuleCalculator {
    pub fn new() -> Self {
        Self
    }
    
    /// Calculate routes from target container to warp container
    pub fn calculate_routes(
        &self,
        destination_cidr: &str,
        gateway_ip: IpAddr,
        interface: Option<String>,
    ) -> Result<Vec<RouteEntry>, RouteError> {
        let network = destination_cidr.parse::<ExternalIpNetwork>()
            .map_err(|e| RouteError::InvalidRoute(e.to_string()))?;
        
        let ip_network = match network {
            ExternalIpNetwork::V4(net) => IpNetwork::V4 {
                addr: net.network(),
                prefix: net.prefix(),
            },
            ExternalIpNetwork::V6(net) => IpNetwork::V6 {
                addr: net.network(),
                prefix: net.prefix(),
            },
        };
        
        let route = RouteEntry {
            destination: ip_network,
            gateway: gateway_ip,
            interface,
            metric: Some(100), // Default metric
        };
        
        Ok(vec![route])
    }
    
    /// Validate route configuration
    pub fn validate_route(&self, route: &RouteEntry) -> Result<(), RouteError> {
        // Basic validation - can be enhanced
        match (&route.destination, &route.gateway) {
            (IpNetwork::V4 { .. }, IpAddr::V4(_)) => Ok(()),
            (IpNetwork::V6 { .. }, IpAddr::V6(_)) => Ok(()),
            _ => Err(RouteError::InvalidRoute(
                "IP version mismatch between destination and gateway".to_string()
            )),
        }
    }
}