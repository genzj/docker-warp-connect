//! Routing rule calculation and validation

use crate::error::RouteError;
use crate::routing::{IpNetwork, RouteEntry};
use ipnetwork::IpNetwork as ExternalIpNetwork;
use std::collections::HashMap;
use std::net::IpAddr;

/// Routing rule calculator
pub struct RoutingRuleCalculator {
    /// Track routes by container ID for cleanup purposes
    container_routes: HashMap<String, Vec<RouteEntry>>,
}

impl RoutingRuleCalculator {
    pub fn new() -> Self {
        Self {
            container_routes: HashMap::new(),
        }
    }

    /// Calculate routes from target container to warp container
    pub fn calculate_routes(
        &self,
        destination_cidr: &str,
        gateway_ip: IpAddr,
        interface: Option<String>,
    ) -> Result<Vec<RouteEntry>, RouteError> {
        let network = destination_cidr
            .parse::<ExternalIpNetwork>()
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

        // Validate the route before returning
        self.validate_route(&route)?;

        Ok(vec![route])
    }

    /// Calculate routes for multiple destination CIDRs
    pub fn calculate_multiple_routes(
        &self,
        destination_cidrs: &[String],
        gateway_ip: IpAddr,
        interface: Option<String>,
    ) -> Result<Vec<RouteEntry>, RouteError> {
        let mut routes = Vec::new();

        for cidr in destination_cidrs {
            let mut cidr_routes = self.calculate_routes(cidr, gateway_ip, interface.clone())?;
            routes.append(&mut cidr_routes);
        }

        // Check for conflicts between routes
        self.detect_route_conflicts(&routes)?;

        Ok(routes)
    }

    /// Track routes for a container (for cleanup purposes)
    pub fn track_container_routes(&mut self, container_id: String, routes: Vec<RouteEntry>) {
        self.container_routes.insert(container_id, routes);
    }

    /// Get routes for cleanup when a container stops
    pub fn get_container_routes_for_cleanup(&self, container_id: &str) -> Vec<RouteEntry> {
        self.container_routes
            .get(container_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Remove tracked routes for a stopped container
    pub fn remove_container_routes(&mut self, container_id: &str) -> Option<Vec<RouteEntry>> {
        self.container_routes.remove(container_id)
    }

    /// Get all tracked containers
    pub fn get_tracked_containers(&self) -> Vec<String> {
        self.container_routes.keys().cloned().collect()
    }

    /// Validate route configuration
    pub fn validate_route(&self, route: &RouteEntry) -> Result<(), RouteError> {
        // Check IP version compatibility
        match (&route.destination, &route.gateway) {
            (IpNetwork::V4 { .. }, IpAddr::V4(_)) => {}
            (IpNetwork::V6 { .. }, IpAddr::V6(_)) => {}
            _ => {
                return Err(RouteError::InvalidRoute(
                    "IP version mismatch between destination and gateway".to_string(),
                ))
            }
        }

        // Validate prefix length
        match &route.destination {
            IpNetwork::V4 { prefix, .. } => {
                if *prefix > 32 {
                    return Err(RouteError::InvalidRoute(format!(
                        "Invalid IPv4 prefix length: {}",
                        prefix
                    )));
                }
            }
            IpNetwork::V6 { prefix, .. } => {
                if *prefix > 128 {
                    return Err(RouteError::InvalidRoute(format!(
                        "Invalid IPv6 prefix length: {}",
                        prefix
                    )));
                }
            }
        }

        // Validate metric
        if let Some(metric) = route.metric {
            if metric == 0 {
                return Err(RouteError::InvalidRoute(
                    "Route metric cannot be zero".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Detect conflicts between routes
    pub fn detect_route_conflicts(&self, routes: &[RouteEntry]) -> Result<(), RouteError> {
        for (i, route1) in routes.iter().enumerate() {
            for (j, route2) in routes.iter().enumerate() {
                if i != j && self.routes_conflict(route1, route2) {
                    return Err(RouteError::InvalidRoute(format!(
                        "Route conflict detected: {} via {} conflicts with {} via {}",
                        route1.destination.addr(),
                        route1.gateway,
                        route2.destination.addr(),
                        route2.gateway
                    )));
                }
            }
        }
        Ok(())
    }

    /// Check if two routes conflict
    fn routes_conflict(&self, route1: &RouteEntry, route2: &RouteEntry) -> bool {
        // Routes conflict if they have the same destination but different gateways
        if route1.destination == route2.destination {
            return route1.gateway != route2.gateway;
        }

        // Check for overlapping networks (simplified check)
        // In a more sophisticated implementation, we would check for subnet overlaps
        false
    }

    /// Calculate default route for all traffic
    pub fn calculate_default_route(&self, gateway_ip: IpAddr) -> Result<RouteEntry, RouteError> {
        let destination = match gateway_ip {
            IpAddr::V4(_) => IpNetwork::V4 {
                addr: std::net::Ipv4Addr::new(0, 0, 0, 0),
                prefix: 0,
            },
            IpAddr::V6(_) => IpNetwork::V6 {
                addr: std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                prefix: 0,
            },
        };

        let route = RouteEntry {
            destination,
            gateway: gateway_ip,
            interface: None,
            metric: Some(200), // Lower priority than specific routes
        };

        self.validate_route(&route)?;
        Ok(route)
    }

    /// Calculate route for specific host
    pub fn calculate_host_route(
        &self,
        host_ip: IpAddr,
        gateway_ip: IpAddr,
    ) -> Result<RouteEntry, RouteError> {
        let destination = match host_ip {
            IpAddr::V4(addr) => IpNetwork::V4 {
                addr,
                prefix: 32, // Host route
            },
            IpAddr::V6(addr) => IpNetwork::V6 {
                addr,
                prefix: 128, // Host route
            },
        };

        let route = RouteEntry {
            destination,
            gateway: gateway_ip,
            interface: None,
            metric: Some(50), // Higher priority than network routes
        };

        self.validate_route(&route)?;
        Ok(route)
    }
}

impl Default for RoutingRuleCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_calculate_routes_ipv4() {
        let calculator = RoutingRuleCalculator::new();
        let gateway = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let routes = calculator
            .calculate_routes("10.0.0.0/8", gateway, None)
            .unwrap();

        assert_eq!(routes.len(), 1);
        let route = &routes[0];
        assert_eq!(route.gateway, gateway);
        assert_eq!(route.destination.prefix(), 8);
        assert_eq!(route.metric, Some(100));
    }

    #[test]
    fn test_calculate_routes_ipv6() {
        let calculator = RoutingRuleCalculator::new();
        let gateway = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));

        let routes = calculator
            .calculate_routes("2001:db8::/32", gateway, None)
            .unwrap();

        assert_eq!(routes.len(), 1);
        let route = &routes[0];
        assert_eq!(route.gateway, gateway);
        assert_eq!(route.destination.prefix(), 32);
    }

    #[test]
    fn test_calculate_routes_invalid_cidr() {
        let calculator = RoutingRuleCalculator::new();
        let gateway = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let result = calculator.calculate_routes("invalid-cidr", gateway, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_route_ip_version_mismatch() {
        let calculator = RoutingRuleCalculator::new();
        let route = RouteEntry {
            destination: IpNetwork::V4 {
                addr: Ipv4Addr::new(10, 0, 0, 0),
                prefix: 8,
            },
            gateway: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            interface: None,
            metric: None,
        };

        let result = calculator.validate_route(&route);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("IP version mismatch"));
    }

    #[test]
    fn test_validate_route_invalid_prefix() {
        let calculator = RoutingRuleCalculator::new();
        let route = RouteEntry {
            destination: IpNetwork::V4 {
                addr: Ipv4Addr::new(10, 0, 0, 0),
                prefix: 33, // Invalid for IPv4
            },
            gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            interface: None,
            metric: None,
        };

        let result = calculator.validate_route(&route);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid IPv4 prefix length"));
    }

    #[test]
    fn test_calculate_multiple_routes() {
        let calculator = RoutingRuleCalculator::new();
        let gateway = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let cidrs = vec!["10.0.0.0/8".to_string(), "172.16.0.0/12".to_string()];

        let routes = calculator
            .calculate_multiple_routes(&cidrs, gateway, None)
            .unwrap();

        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].destination.prefix(), 8);
        assert_eq!(routes[1].destination.prefix(), 12);
    }

    #[test]
    fn test_track_container_routes() {
        let mut calculator = RoutingRuleCalculator::new();
        let gateway = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let routes = calculator
            .calculate_routes("10.0.0.0/8", gateway, None)
            .unwrap();

        calculator.track_container_routes("container-123".to_string(), routes.clone());

        let tracked_routes = calculator.get_container_routes_for_cleanup("container-123");
        assert_eq!(tracked_routes.len(), 1);
        assert_eq!(tracked_routes[0].gateway, gateway);
    }

    #[test]
    fn test_remove_container_routes() {
        let mut calculator = RoutingRuleCalculator::new();
        let gateway = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let routes = calculator
            .calculate_routes("10.0.0.0/8", gateway, None)
            .unwrap();

        calculator.track_container_routes("container-123".to_string(), routes);

        let removed_routes = calculator.remove_container_routes("container-123");
        assert!(removed_routes.is_some());
        assert_eq!(removed_routes.unwrap().len(), 1);

        let tracked_routes = calculator.get_container_routes_for_cleanup("container-123");
        assert_eq!(tracked_routes.len(), 0);
    }

    #[test]
    fn test_calculate_default_route() {
        let calculator = RoutingRuleCalculator::new();
        let gateway = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let route = calculator.calculate_default_route(gateway).unwrap();

        assert_eq!(route.gateway, gateway);
        assert_eq!(route.destination.prefix(), 0);
        assert_eq!(route.metric, Some(200));
    }

    #[test]
    fn test_calculate_host_route() {
        let calculator = RoutingRuleCalculator::new();
        let host = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let gateway = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let route = calculator.calculate_host_route(host, gateway).unwrap();

        assert_eq!(route.gateway, gateway);
        assert_eq!(route.destination.addr(), host);
        assert_eq!(route.destination.prefix(), 32);
        assert_eq!(route.metric, Some(50));
    }

    #[test]
    fn test_routes_conflict_detection() {
        let calculator = RoutingRuleCalculator::new();
        let gateway1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let gateway2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        let route1 = RouteEntry {
            destination: IpNetwork::V4 {
                addr: Ipv4Addr::new(10, 0, 0, 0),
                prefix: 8,
            },
            gateway: gateway1,
            interface: None,
            metric: None,
        };

        let route2 = RouteEntry {
            destination: IpNetwork::V4 {
                addr: Ipv4Addr::new(10, 0, 0, 0),
                prefix: 8,
            },
            gateway: gateway2,
            interface: None,
            metric: None,
        };

        let routes = vec![route1, route2];
        let result = calculator.detect_route_conflicts(&routes);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Route conflict detected"));
    }
}
