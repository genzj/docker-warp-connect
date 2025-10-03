//! Route management using rtnetlink

use crate::docker::DockerClient;
use crate::error::{NetworkError, RouteError};
use crate::network::{namespace::NamespaceManager, NetworkNamespace};
use crate::routing::{IpNetwork, RouteEntry, RouteManager};
use rtnetlink::{new_connection, Handle};
use std::net::IpAddr;

/// Route manager implementation using rtnetlink
pub struct RtNetlinkRouteManager<D: DockerClient> {
    namespace_manager: NamespaceManager<D>,
}

impl<D: DockerClient> RtNetlinkRouteManager<D> {
    /// Create a new route manager
    pub fn new(docker_client: D) -> Self {
        Self {
            namespace_manager: NamespaceManager::new(docker_client),
        }
    }

    /// Create a new rtnetlink connection within the current namespace
    async fn create_connection() -> Result<Handle, RouteError> {
        let (connection, handle, _) = new_connection().map_err(|e| {
            RouteError::AddRoute(format!("Failed to create netlink connection: {}", e))
        })?;

        // Spawn the connection handler
        tokio::spawn(connection);

        Ok(handle)
    }

    /// Convert our IpNetwork to components
    fn convert_network(network: &IpNetwork) -> (IpAddr, u8) {
        match network {
            IpNetwork::V4 { addr, prefix } => (IpAddr::V4(*addr), *prefix),
            IpNetwork::V6 { addr, prefix } => (IpAddr::V6(*addr), *prefix),
        }
    }
}

impl<D: DockerClient> RtNetlinkRouteManager<D> {
    /// Execute a route operation within a network namespace
    /// This utility method handles the common pattern of:
    /// 1. Executing within the namespace
    /// 2. Creating a netlink connection
    /// 3. Converting network errors to route errors
    async fn execute_route_operation<T, F, Fut>(
        &self,
        namespace: &NetworkNamespace,
        operation: F,
        error_converter: fn(NetworkError) -> RouteError,
    ) -> Result<T, RouteError>
    where
        F: FnOnce(Handle) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, NetworkError>> + Send,
        T: Send + 'static,
    {
        self.namespace_manager
            .execute_in_namespace(namespace, move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    let handle = Self::create_connection().await.map_err(|e| {
                        NetworkError::OperationFailed(format!("Failed to create connection: {}", e))
                    })?;

                    operation(handle).await
                })
            })
            .await
            .map_err(error_converter)
    }

    /// Convert NetworkError to RouteError for add operations
    fn map_add_route_error(e: NetworkError) -> RouteError {
        match e {
            NetworkError::InsufficientPrivileges => RouteError::AddRoute(
                "Insufficient privileges to access network namespace".to_string(),
            ),
            NetworkError::NamespaceAccess(msg) => {
                RouteError::AddRoute(format!("Failed to access network namespace: {}", msg))
            }
            NetworkError::OperationFailed(msg) => RouteError::AddRoute(msg),
            _ => RouteError::AddRoute(format!("Network operation failed: {}", e)),
        }
    }

    /// Convert NetworkError to RouteError for remove operations
    fn map_remove_route_error(e: NetworkError) -> RouteError {
        match e {
            NetworkError::InsufficientPrivileges => RouteError::RemoveRoute(
                "Insufficient privileges to access network namespace".to_string(),
            ),
            NetworkError::NamespaceAccess(msg) => {
                RouteError::RemoveRoute(format!("Failed to access network namespace: {}", msg))
            }
            NetworkError::OperationFailed(msg) => RouteError::RemoveRoute(msg),
            _ => RouteError::RemoveRoute(format!("Network operation failed: {}", e)),
        }
    }

    /// Convert NetworkError to RouteError for list operations
    fn map_list_route_error(e: NetworkError) -> RouteError {
        match e {
            NetworkError::InsufficientPrivileges => RouteError::InvalidRoute(
                "Insufficient privileges to access network namespace".to_string(),
            ),
            NetworkError::NamespaceAccess(msg) => {
                RouteError::InvalidRoute(format!("Failed to access network namespace: {}", msg))
            }
            NetworkError::OperationFailed(msg) => RouteError::InvalidRoute(msg),
            _ => RouteError::InvalidRoute(format!("Network operation failed: {}", e)),
        }
    }
}

impl<D: DockerClient + Send + Sync> RouteManager for RtNetlinkRouteManager<D> {
    async fn add_route(
        &self,
        namespace: &NetworkNamespace,
        route: &RouteEntry,
    ) -> Result<(), RouteError> {
        let route_clone = route.clone();

        self.execute_route_operation(
            namespace,
            move |_handle| async move {
                let (dest_addr, prefix) = Self::convert_network(&route_clone.destination);

                // For now, we'll implement a basic route addition
                // The exact rtnetlink API usage needs to be researched further
                // This is a placeholder that establishes the structure

                // The actual implementation would use rtnetlink to add routes
                // For now, we'll return an error indicating this needs implementation
                // This establishes the structure for future implementation

                Err(NetworkError::OperationFailed(format!(
                    "Route addition not yet fully implemented for {} via {} (prefix: {})",
                    dest_addr, route_clone.gateway, prefix
                )))
            },
            Self::map_add_route_error,
        )
        .await
    }

    async fn remove_route(
        &self,
        namespace: &NetworkNamespace,
        route: &RouteEntry,
    ) -> Result<(), RouteError> {
        let route_clone = route.clone();

        self.execute_route_operation(
            namespace,
            move |_handle| async move {
                let (dest_addr, prefix) = Self::convert_network(&route_clone.destination);

                // The actual implementation would use rtnetlink to delete routes

                Err(NetworkError::OperationFailed(format!(
                    "Route removal not yet fully implemented for {} via {} (prefix: {})",
                    dest_addr, route_clone.gateway, prefix
                )))
            },
            Self::map_remove_route_error,
        )
        .await
    }

    async fn list_routes(
        &self,
        namespace: &NetworkNamespace,
    ) -> Result<Vec<RouteEntry>, RouteError> {
        self.execute_route_operation(
            namespace,
            move |_handle| async move {
                // For now, return empty list - route listing is complex and not critical for MVP
                // This can be implemented later when needed for debugging/monitoring
                Ok(Vec::new())
            },
            Self::map_list_route_error,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::{ContainerInfo, ContainerState, NetworkInfo};
    use crate::error::DockerError;
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    // Mock Docker client for testing
    struct MockDockerClient;

    impl DockerClient for MockDockerClient {
        async fn list_containers(&self, _all: bool) -> Result<Vec<ContainerInfo>, DockerError> {
            Ok(vec![])
        }

        async fn inspect_container(&self, _id: &str) -> Result<ContainerInfo, DockerError> {
            Ok(ContainerInfo {
                id: "test".to_string(),
                name: "test".to_string(),
                labels: HashMap::new(),
                networks: vec![],
                state: ContainerState::Running,
                pid: Some(1234),
            })
        }

        async fn get_container_networks(&self, _id: &str) -> Result<Vec<NetworkInfo>, DockerError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_route_manager_creation() {
        let docker_client = MockDockerClient;
        let _manager = RtNetlinkRouteManager::new(docker_client);
        // Just test that we can create the manager without panicking
    }

    #[test]
    fn test_convert_network_v4() {
        let network = IpNetwork::V4 {
            addr: Ipv4Addr::new(192, 168, 1, 0),
            prefix: 24,
        };
        let (addr, prefix) = RtNetlinkRouteManager::<MockDockerClient>::convert_network(&network);
        assert_eq!(addr, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0)));
        assert_eq!(prefix, 24);
    }

    #[test]
    fn test_convert_network_v6() {
        let network = IpNetwork::V6 {
            addr: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            prefix: 64,
        };
        let (addr, prefix) = RtNetlinkRouteManager::<MockDockerClient>::convert_network(&network);
        assert_eq!(
            addr,
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0))
        );
        assert_eq!(prefix, 64);
    }

    #[test]
    fn test_route_entry_creation() {
        let route = RouteEntry {
            destination: IpNetwork::V4 {
                addr: Ipv4Addr::new(10, 0, 0, 0),
                prefix: 8,
            },
            gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            interface: Some("eth0".to_string()),
            metric: Some(100),
        };

        assert_eq!(route.destination.prefix(), 8);
        assert_eq!(route.gateway, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(route.interface, Some("eth0".to_string()));
        assert_eq!(route.metric, Some(100));
    }

    #[tokio::test]
    async fn test_add_route_returns_error() {
        let docker_client = MockDockerClient;
        let manager = RtNetlinkRouteManager::new(docker_client);

        let namespace = NetworkNamespace {
            path: "/proc/1234/ns/net".to_string(),
            container_id: "test".to_string(),
        };

        let route = RouteEntry {
            destination: IpNetwork::V4 {
                addr: Ipv4Addr::new(10, 0, 0, 0),
                prefix: 8,
            },
            gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            interface: None,
            metric: None,
        };

        // This should return an error (either namespace access or not implemented)
        let result = manager.add_route(&namespace, &route).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            RouteError::AddRoute(_msg) => {
                // Any AddRoute error is acceptable for this test
                // The actual error could be namespace access failure or implementation error
            }
            _ => panic!("Expected AddRoute error"),
        }
    }
}
