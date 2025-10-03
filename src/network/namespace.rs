//! Network namespace operations

use crate::docker::{DockerClient, NetworkInfo};
use crate::error::NetworkError;
use crate::network::{NetworkManager, NetworkNamespace};
use netns_rs::NetNs;
use std::net::IpAddr;
use std::path::Path;

/// Network namespace manager implementation
pub struct NamespaceManager<D: DockerClient> {
    docker_client: D,
}

impl<D: DockerClient> NamespaceManager<D> {
    /// Create a new namespace manager with a Docker client
    #[allow(dead_code)]
    pub fn new(docker_client: D) -> Self {
        Self { docker_client }
    }

    /// Get container PID from Docker inspect
    #[allow(dead_code)]
    async fn get_container_pid(&self, container_id: &str) -> Result<u32, NetworkError> {
        let container = self
            .docker_client
            .inspect_container(container_id)
            .await
            .map_err(|e| {
                NetworkError::OperationFailed(format!("Failed to inspect container: {}", e))
            })?;

        // Check if container is running
        match container.state {
            crate::docker::ContainerState::Running => {}
            _ => {
                return Err(NetworkError::OperationFailed(format!(
                    "Container {} is not running",
                    container_id
                )))
            }
        }

        // Get PID from container info
        let pid = container.pid.ok_or_else(|| {
            NetworkError::OperationFailed(format!(
                "Container {} has no PID information",
                container_id
            ))
        })?;

        // Convert i64 to u32, checking for valid range
        if pid < 0 || pid > u32::MAX as i64 {
            return Err(NetworkError::OperationFailed(format!(
                "Container {} has invalid PID: {}",
                container_id, pid
            )));
        }

        Ok(pid as u32)
    }

    /// Execute a function within a network namespace
    #[allow(dead_code)]
    pub async fn execute_in_namespace<F, R>(
        &self,
        namespace: &NetworkNamespace,
        func: F,
    ) -> Result<R, NetworkError>
    where
        F: FnOnce() -> Result<R, NetworkError> + Send,
        R: Send,
    {
        // Check if namespace path exists
        if !Path::new(&namespace.path).exists() {
            return Err(NetworkError::NamespaceAccess(format!(
                "Namespace path does not exist: {}",
                namespace.path
            )));
        }

        // Try to get the namespace
        let ns = NetNs::get(&namespace.path).map_err(|e| {
            // Check for permission errors specifically
            if e.to_string().contains("Permission denied")
                || e.to_string().contains("Operation not permitted")
            {
                NetworkError::InsufficientPrivileges
            } else {
                NetworkError::NamespaceAccess(e.to_string())
            }
        })?;

        // Enter the namespace
        let _guard = ns.enter().map_err(|e| {
            if e.to_string().contains("Permission denied")
                || e.to_string().contains("Operation not permitted")
            {
                NetworkError::InsufficientPrivileges
            } else {
                NetworkError::NamespaceAccess(e.to_string())
            }
        })?;

        // Execute the function within the namespace
        func()
    }
}

impl<D: DockerClient> NetworkManager for NamespaceManager<D> {
    async fn get_container_namespace(
        &self,
        container_id: &str,
    ) -> Result<NetworkNamespace, NetworkError> {
        let pid = self.get_container_pid(container_id).await?;
        let ns_path = format!("/proc/{}/ns/net", pid);

        // Verify the namespace path exists
        if !Path::new(&ns_path).exists() {
            return Err(NetworkError::NamespaceAccess(format!(
                "Network namespace not found at {}",
                ns_path
            )));
        }

        Ok(NetworkNamespace {
            path: ns_path,
            container_id: container_id.to_string(),
        })
    }

    async fn get_container_networks(
        &self,
        container_id: &str,
    ) -> Result<Vec<NetworkInfo>, NetworkError> {
        self.docker_client
            .get_container_networks(container_id)
            .await
            .map_err(|e| {
                NetworkError::OperationFailed(format!("Failed to get container networks: {}", e))
            })
    }

    async fn resolve_container_ip(
        &self,
        container_id: &str,
        network: Option<&str>,
    ) -> Result<IpAddr, NetworkError> {
        let networks = self.get_container_networks(container_id).await?;

        if networks.is_empty() {
            return Err(NetworkError::NetworkNotFound {
                container_id: container_id.to_string(),
            });
        }

        // If a specific network is requested, find it
        if let Some(network_name) = network {
            for net in &networks {
                if net.name == network_name {
                    return Ok(net.ip_address);
                }
            }
            return Err(NetworkError::NetworkNotFound {
                container_id: container_id.to_string(),
            });
        }

        // If no specific network requested and only one network is attached, return the first available IP
        if networks.len() == 1 {
            return Ok(networks[0].ip_address);
        }

        return Err(NetworkError::MultipleNetworksExist {
            container_id: container_id.to_string(),
        });
    }
}

impl<D: DockerClient> NamespaceManager<D> {
    /// Resolve container IP with network preference from container labels
    /// This method inspects the container's labels to determine network preference
    pub async fn resolve_container_ip_with_preference(
        &self,
        container_id: &str,
        network_preference_label: &str,
    ) -> Result<IpAddr, NetworkError> {
        // Get container info to access labels
        let container = self
            .docker_client
            .inspect_container(container_id)
            .await
            .map_err(|e| {
                NetworkError::OperationFailed(format!("Failed to inspect container: {}", e))
            })?;

        let networks = container.networks;

        if networks.is_empty() {
            return Err(NetworkError::NetworkNotFound {
                container_id: container_id.to_string(),
            });
        }

        // Check if container has network preference label
        if let Some(preferred_network) = container.labels.get(network_preference_label) {
            // If container has multiple networks and a preference is specified
            if networks.len() > 1 {
                for net in &networks {
                    if net.name == *preferred_network {
                        return Ok(net.ip_address);
                    }
                }
                // If preferred network not found, return error as per requirement 2.5.3
                return Err(NetworkError::OperationFailed(format!(
                    "Preferred network '{}' not found for container {}",
                    preferred_network, container_id
                )));
            }
        } else if networks.len() > 1 {
            // If container has multiple networks but no preference label, return error as per requirement 2.5.3
            return Err(NetworkError::OperationFailed(format!(
                "Container {} has multiple networks but no network preference label '{}'",
                container_id, network_preference_label
            )));
        }

        // If container has only one network or preference matches, return the IP
        Ok(networks[0].ip_address)
    }

    /// Select the appropriate network from a list based on preference
    pub fn select_network_by_preference<'a>(
        &self,
        networks: &'a [NetworkInfo],
        preference: Option<&str>,
    ) -> Result<&'a NetworkInfo, NetworkError> {
        if networks.is_empty() {
            return Err(NetworkError::OperationFailed(
                "No networks available".to_string(),
            ));
        }

        if let Some(preferred_network) = preference {
            // Find the preferred network
            for net in networks {
                if net.name == preferred_network {
                    return Ok(net);
                }
            }
            // If preferred network not found, return error
            return Err(NetworkError::OperationFailed(format!(
                "Preferred network '{}' not found",
                preferred_network
            )));
        }

        // If no preference specified and multiple networks, this is an error condition
        if networks.len() > 1 {
            return Err(NetworkError::OperationFailed(
                "Multiple networks available but no preference specified".to_string(),
            ));
        }

        // Return the single network
        Ok(&networks[0])
    }

    /// Get all network information for a container with detailed analysis
    pub async fn analyze_container_networks(
        &self,
        container_id: &str,
    ) -> Result<ContainerNetworkAnalysis, NetworkError> {
        let container = self
            .docker_client
            .inspect_container(container_id)
            .await
            .map_err(|e| {
                NetworkError::OperationFailed(format!("Failed to inspect container: {}", e))
            })?;

        let networks = container.networks;
        let has_multiple_networks = networks.len() > 1;
        let network_names: Vec<String> = networks.iter().map(|n| n.name.clone()).collect();

        Ok(ContainerNetworkAnalysis {
            container_id: container_id.to_string(),
            container_name: container.name,
            networks,
            has_multiple_networks,
            network_names,
            labels: container.labels,
        })
    }
}

/// Container network analysis result
#[derive(Debug, Clone)]
pub struct ContainerNetworkAnalysis {
    pub container_id: String,
    pub container_name: String,
    pub networks: Vec<NetworkInfo>,
    pub has_multiple_networks: bool,
    pub network_names: Vec<String>,
    pub labels: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::{ContainerInfo, ContainerState};
    use crate::error::DockerError;
    use ipnetwork::IpNetwork;
    use std::collections::HashMap;
    use std::net::IpAddr;
    use std::str::FromStr;

    // Mock Docker client for testing
    struct MockDockerClient {
        containers: HashMap<String, ContainerInfo>,
        should_fail: bool,
    }

    impl MockDockerClient {
        fn new() -> Self {
            Self {
                containers: HashMap::new(),
                should_fail: false,
            }
        }

        fn add_container(&mut self, container: ContainerInfo) {
            self.containers.insert(container.id.clone(), container);
        }

        fn set_should_fail(&mut self, should_fail: bool) {
            self.should_fail = should_fail;
        }
    }

    impl DockerClient for MockDockerClient {
        async fn list_containers(&self, _all: bool) -> Result<Vec<ContainerInfo>, DockerError> {
            if self.should_fail {
                return Err(DockerError::ApiError("Mock failure".to_string()));
            }
            Ok(self.containers.values().cloned().collect())
        }

        async fn inspect_container(&self, id: &str) -> Result<ContainerInfo, DockerError> {
            if self.should_fail {
                return Err(DockerError::ApiError("Mock failure".to_string()));
            }

            self.containers
                .get(id)
                .cloned()
                .ok_or_else(|| DockerError::ContainerNotFound {
                    container_id: id.to_string(),
                })
        }

        async fn get_container_networks(&self, id: &str) -> Result<Vec<NetworkInfo>, DockerError> {
            if self.should_fail {
                return Err(DockerError::ApiError("Mock failure".to_string()));
            }

            let container =
                self.containers
                    .get(id)
                    .ok_or_else(|| DockerError::ContainerNotFound {
                        container_id: id.to_string(),
                    })?;

            Ok(container.networks.clone())
        }
    }

    fn create_test_container(id: &str, name: &str, state: ContainerState) -> ContainerInfo {
        let ip = IpAddr::from_str("192.168.1.10").unwrap();
        let subnet = IpNetwork::new(ip, 24).unwrap();

        ContainerInfo {
            id: id.to_string(),
            name: name.to_string(),
            labels: HashMap::new(),
            networks: vec![NetworkInfo {
                name: "bridge".to_string(),
                ip_address: ip,
                gateway: Some(IpAddr::from_str("192.168.1.1").unwrap()),
                subnet,
            }],
            state,
            pid: Some(30),
        }
    }

    #[tokio::test]
    async fn test_get_container_networks() {
        let mut mock_client = MockDockerClient::new();
        let container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let networks = manager.get_container_networks("test-123").await.unwrap();

        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].name, "bridge");
        assert_eq!(
            networks[0].ip_address,
            IpAddr::from_str("192.168.1.10").unwrap()
        );
    }

    #[tokio::test]
    async fn test_resolve_container_ip_default_network() {
        let mut mock_client = MockDockerClient::new();
        let container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let ip = manager
            .resolve_container_ip("test-123", None)
            .await
            .unwrap();

        assert_eq!(ip, IpAddr::from_str("192.168.1.10").unwrap());
    }

    #[tokio::test]
    async fn test_resolve_container_ip_specific_network() {
        let mut mock_client = MockDockerClient::new();
        let mut container =
            create_test_container("test-123", "test-container", ContainerState::Running);

        // Add a second network
        let ip2 = IpAddr::from_str("10.0.0.5").unwrap();
        let subnet2 = IpNetwork::new(ip2, 16).unwrap();
        container.networks.push(NetworkInfo {
            name: "custom".to_string(),
            ip_address: ip2,
            gateway: Some(IpAddr::from_str("10.0.0.1").unwrap()),
            subnet: subnet2,
        });

        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);

        // Test specific network resolution
        let ip = manager
            .resolve_container_ip("test-123", Some("custom"))
            .await
            .unwrap();
        assert_eq!(ip, IpAddr::from_str("10.0.0.5").unwrap());

        // Test bridge network resolution
        let ip = manager
            .resolve_container_ip("test-123", Some("bridge"))
            .await
            .unwrap();
        assert_eq!(ip, IpAddr::from_str("192.168.1.10").unwrap());
    }

    #[tokio::test]
    async fn test_resolve_container_ip_network_not_found() {
        let mut mock_client = MockDockerClient::new();
        let container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let result = manager
            .resolve_container_ip("test-123", Some("nonexistent"))
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::NetworkNotFound { container_id } => {
                assert_eq!(container_id, "test-123");
            }
            _ => panic!("Expected NetworkNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_resolve_container_ip_no_networks() {
        let mut mock_client = MockDockerClient::new();
        let mut container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        container.networks.clear(); // Remove all networks
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let result = manager.resolve_container_ip("test-123", None).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::NetworkNotFound { container_id } => {
                assert_eq!(container_id, "test-123");
            }
            _ => panic!("Expected NetworkNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_container_pid_not_running() {
        let mut mock_client = MockDockerClient::new();
        let container =
            create_test_container("test-123", "test-container", ContainerState::Stopped);
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let result = manager.get_container_pid("test-123").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("not running"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_get_container_pid_container_not_found() {
        let mock_client = MockDockerClient::new();
        let manager = NamespaceManager::new(mock_client);
        let result = manager.get_container_pid("nonexistent").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("Failed to inspect container"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_docker_client_failure() {
        let mut mock_client = MockDockerClient::new();
        mock_client.set_should_fail(true);

        let manager = NamespaceManager::new(mock_client);
        let result = manager.get_container_networks("test-123").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("Failed to get container networks"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_resolve_container_ip_with_preference_single_network() {
        let mut mock_client = MockDockerClient::new();
        let container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let ip = manager
            .resolve_container_ip_with_preference("test-123", "network.warp.network")
            .await
            .unwrap();

        assert_eq!(ip, IpAddr::from_str("192.168.1.10").unwrap());
    }

    #[tokio::test]
    async fn test_resolve_container_ip_with_preference_multiple_networks_with_label() {
        let mut mock_client = MockDockerClient::new();
        let mut container =
            create_test_container("test-123", "test-container", ContainerState::Running);

        // Add a second network
        let ip2 = IpAddr::from_str("10.0.0.5").unwrap();
        let subnet2 = IpNetwork::new(ip2, 16).unwrap();
        container.networks.push(NetworkInfo {
            name: "custom".to_string(),
            ip_address: ip2,
            gateway: Some(IpAddr::from_str("10.0.0.1").unwrap()),
            subnet: subnet2,
        });

        // Add network preference label
        container
            .labels
            .insert("network.warp.network".to_string(), "custom".to_string());

        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let ip = manager
            .resolve_container_ip_with_preference("test-123", "network.warp.network")
            .await
            .unwrap();

        assert_eq!(ip, IpAddr::from_str("10.0.0.5").unwrap());
    }

    #[tokio::test]
    async fn test_resolve_container_ip_with_preference_multiple_networks_no_label() {
        let mut mock_client = MockDockerClient::new();
        let mut container =
            create_test_container("test-123", "test-container", ContainerState::Running);

        // Add a second network
        let ip2 = IpAddr::from_str("10.0.0.5").unwrap();
        let subnet2 = IpNetwork::new(ip2, 16).unwrap();
        container.networks.push(NetworkInfo {
            name: "custom".to_string(),
            ip_address: ip2,
            gateway: Some(IpAddr::from_str("10.0.0.1").unwrap()),
            subnet: subnet2,
        });

        // No network preference label
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let result = manager
            .resolve_container_ip_with_preference("test-123", "network.warp.network")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("multiple networks but no network preference label"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_resolve_container_ip_with_preference_invalid_network() {
        let mut mock_client = MockDockerClient::new();
        let mut container =
            create_test_container("test-123", "test-container", ContainerState::Running);

        // Add a second network
        let ip2 = IpAddr::from_str("10.0.0.5").unwrap();
        let subnet2 = IpNetwork::new(ip2, 16).unwrap();
        container.networks.push(NetworkInfo {
            name: "custom".to_string(),
            ip_address: ip2,
            gateway: Some(IpAddr::from_str("10.0.0.1").unwrap()),
            subnet: subnet2,
        });

        // Add network preference label with invalid network name
        container.labels.insert(
            "network.warp.network".to_string(),
            "nonexistent".to_string(),
        );

        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let result = manager
            .resolve_container_ip_with_preference("test-123", "network.warp.network")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("Preferred network 'nonexistent' not found"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_select_network_by_preference_single_network() {
        let mock_client = MockDockerClient::new();
        let manager = NamespaceManager::new(mock_client);

        let ip = IpAddr::from_str("192.168.1.10").unwrap();
        let subnet = IpNetwork::new(ip, 24).unwrap();
        let networks = vec![NetworkInfo {
            name: "bridge".to_string(),
            ip_address: ip,
            gateway: Some(IpAddr::from_str("192.168.1.1").unwrap()),
            subnet,
        }];

        let selected = manager
            .select_network_by_preference(&networks, None)
            .unwrap();
        assert_eq!(selected.name, "bridge");
        assert_eq!(selected.ip_address, ip);
    }

    #[tokio::test]
    async fn test_select_network_by_preference_multiple_networks_with_preference() {
        let mock_client = MockDockerClient::new();
        let manager = NamespaceManager::new(mock_client);

        let ip1 = IpAddr::from_str("192.168.1.10").unwrap();
        let subnet1 = IpNetwork::new(ip1, 24).unwrap();
        let ip2 = IpAddr::from_str("10.0.0.5").unwrap();
        let subnet2 = IpNetwork::new(ip2, 16).unwrap();

        let networks = vec![
            NetworkInfo {
                name: "bridge".to_string(),
                ip_address: ip1,
                gateway: Some(IpAddr::from_str("192.168.1.1").unwrap()),
                subnet: subnet1,
            },
            NetworkInfo {
                name: "custom".to_string(),
                ip_address: ip2,
                gateway: Some(IpAddr::from_str("10.0.0.1").unwrap()),
                subnet: subnet2,
            },
        ];

        let selected = manager
            .select_network_by_preference(&networks, Some("custom"))
            .unwrap();
        assert_eq!(selected.name, "custom");
        assert_eq!(selected.ip_address, ip2);
    }

    #[tokio::test]
    async fn test_select_network_by_preference_multiple_networks_no_preference() {
        let mock_client = MockDockerClient::new();
        let manager = NamespaceManager::new(mock_client);

        let ip1 = IpAddr::from_str("192.168.1.10").unwrap();
        let subnet1 = IpNetwork::new(ip1, 24).unwrap();
        let ip2 = IpAddr::from_str("10.0.0.5").unwrap();
        let subnet2 = IpNetwork::new(ip2, 16).unwrap();

        let networks = vec![
            NetworkInfo {
                name: "bridge".to_string(),
                ip_address: ip1,
                gateway: Some(IpAddr::from_str("192.168.1.1").unwrap()),
                subnet: subnet1,
            },
            NetworkInfo {
                name: "custom".to_string(),
                ip_address: ip2,
                gateway: Some(IpAddr::from_str("10.0.0.1").unwrap()),
                subnet: subnet2,
            },
        ];

        let result = manager.select_network_by_preference(&networks, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("Multiple networks available but no preference specified"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_select_network_by_preference_invalid_preference() {
        let mock_client = MockDockerClient::new();
        let manager = NamespaceManager::new(mock_client);

        let ip = IpAddr::from_str("192.168.1.10").unwrap();
        let subnet = IpNetwork::new(ip, 24).unwrap();
        let networks = vec![NetworkInfo {
            name: "bridge".to_string(),
            ip_address: ip,
            gateway: Some(IpAddr::from_str("192.168.1.1").unwrap()),
            subnet,
        }];

        let result = manager.select_network_by_preference(&networks, Some("nonexistent"));
        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("Preferred network 'nonexistent' not found"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_analyze_container_networks() {
        let mut mock_client = MockDockerClient::new();
        let mut container =
            create_test_container("test-123", "test-container", ContainerState::Running);

        // Add a second network
        let ip2 = IpAddr::from_str("10.0.0.5").unwrap();
        let subnet2 = IpNetwork::new(ip2, 16).unwrap();
        container.networks.push(NetworkInfo {
            name: "custom".to_string(),
            ip_address: ip2,
            gateway: Some(IpAddr::from_str("10.0.0.1").unwrap()),
            subnet: subnet2,
        });

        // Add some labels
        container
            .labels
            .insert("app".to_string(), "test".to_string());
        container
            .labels
            .insert("network.warp.network".to_string(), "custom".to_string());

        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let analysis = manager
            .analyze_container_networks("test-123")
            .await
            .unwrap();

        assert_eq!(analysis.container_id, "test-123");
        assert_eq!(analysis.container_name, "test-container");
        assert_eq!(analysis.networks.len(), 2);
        assert!(analysis.has_multiple_networks);
        assert_eq!(analysis.network_names, vec!["bridge", "custom"]);
        assert_eq!(analysis.labels.get("app"), Some(&"test".to_string()));
        assert_eq!(
            analysis.labels.get("network.warp.network"),
            Some(&"custom".to_string())
        );
    }

    #[tokio::test]
    async fn test_analyze_container_networks_single_network() {
        let mut mock_client = MockDockerClient::new();
        let container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let analysis = manager
            .analyze_container_networks("test-123")
            .await
            .unwrap();

        assert_eq!(analysis.container_id, "test-123");
        assert_eq!(analysis.container_name, "test-container");
        assert_eq!(analysis.networks.len(), 1);
        assert!(!analysis.has_multiple_networks);
        assert_eq!(analysis.network_names, vec!["bridge"]);
    }

    #[tokio::test]
    async fn test_get_container_pid_success() {
        let mut mock_client = MockDockerClient::new();
        let container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let pid = manager.get_container_pid("test-123").await.unwrap();

        assert_eq!(pid, 30);
    }

    #[tokio::test]
    async fn test_get_container_pid_no_pid() {
        let mut mock_client = MockDockerClient::new();
        let mut container =
            create_test_container("test-123", "test-container", ContainerState::Running);
        container.pid = None; // Remove PID
        mock_client.add_container(container);

        let manager = NamespaceManager::new(mock_client);
        let result = manager.get_container_pid("test-123").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::OperationFailed(msg) => {
                assert!(msg.contains("has no PID information"));
            }
            _ => panic!("Expected OperationFailed error"),
        }
    }
}
