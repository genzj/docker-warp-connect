//! Docker API integration module
//!
//! Handles Docker API connections, event monitoring, and container classification

use crate::error::{DockerError, EventError, HandlerError};
use bollard::models::{ContainerInspectResponse, ContainerSummary};
use bollard::query_parameters::{InspectContainerOptions, ListContainersOptions};
use bollard::Docker;
use ipnetwork::IpNetwork;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;

pub mod classifier;
pub mod events;

/// Container information structure
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub labels: HashMap<String, String>,
    pub networks: Vec<NetworkInfo>,
    pub state: ContainerState,
    pub pid: Option<i64>,
}

/// Network information for containers
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkInfo {
    pub name: String,
    pub ip_address: IpAddr,
    pub gateway: Option<IpAddr>,
    pub subnet: IpNetwork,
}

/// Container state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Starting,
    Running,
    Stopped,
}

/// Container start event
#[derive(Debug, Clone)]
pub struct ContainerStartEvent {
    pub container: ContainerInfo,
}

/// Docker client wrapper trait for testability
pub trait DockerClient: Send + Sync {
    /// List all containers
    fn list_containers(
        &self,
        all: bool,
    ) -> impl std::future::Future<Output = Result<Vec<ContainerInfo>, DockerError>> + Send;

    /// Inspect a specific container
    fn inspect_container(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<ContainerInfo, DockerError>> + Send;

    /// Get container networks and IP addresses
    fn get_container_networks(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<NetworkInfo>, DockerError>> + Send;
}

/// Bollard-based Docker client implementation
pub struct BollardDockerClient {
    docker: Docker,
}

impl BollardDockerClient {
    /// Create a new Docker client with default connection
    pub fn new() -> Result<Self, DockerError> {
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        Ok(Self { docker })
    }

    /// Create a new Docker client with custom socket path
    pub fn with_socket(socket_path: &str) -> Result<Self, DockerError> {
        let docker = Docker::connect_with_socket(socket_path, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        Ok(Self { docker })
    }

    /// Create a new Docker client with HTTP connection
    pub fn with_http(url: &str) -> Result<Self, DockerError> {
        let docker = Docker::connect_with_http(url, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        Ok(Self { docker })
    }

    /// Convert bollard container summary to our ContainerInfo
    fn convert_container_summary(
        &self,
        summary: ContainerSummary,
    ) -> Result<ContainerInfo, DockerError> {
        let id = summary.id.unwrap_or_default();
        let names = summary.names.unwrap_or_default();
        let name = names
            .first()
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();

        let labels = summary.labels.unwrap_or_default();
        let state_enum = summary.state;

        let state = if let Some(state_enum) = state_enum {
            use bollard::models::ContainerSummaryStateEnum;
            match state_enum {
                ContainerSummaryStateEnum::RUNNING => ContainerState::Running,
                ContainerSummaryStateEnum::CREATED | ContainerSummaryStateEnum::RESTARTING => {
                    ContainerState::Starting
                }
                ContainerSummaryStateEnum::PAUSED
                | ContainerSummaryStateEnum::EXITED
                | ContainerSummaryStateEnum::DEAD => ContainerState::Stopped,
                _ => ContainerState::Stopped,
            }
        } else {
            ContainerState::Stopped
        };

        // Networks will be populated separately via inspect_container
        let networks = Vec::new();

        Ok(ContainerInfo {
            id,
            name,
            labels,
            networks,
            state,
            pid: None,
        })
    }

    /// Convert bollard container inspect response to our ContainerInfo
    fn convert_container_inspect(
        &self,
        inspect: ContainerInspectResponse,
    ) -> Result<ContainerInfo, DockerError> {
        let id = inspect.id.unwrap_or_default();
        let name = inspect
            .name
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();

        let labels = inspect.config.and_then(|c| c.labels).unwrap_or_default();

        let state_info = inspect.state.unwrap_or_default();
        let state = if state_info.running.unwrap_or(false) {
            ContainerState::Running
        } else if state_info.restarting.unwrap_or(false) {
            ContainerState::Starting
        } else {
            ContainerState::Stopped
        };
        let pid = state_info.pid;

        // Extract network information
        let mut networks = Vec::new();
        if let Some(network_settings) = inspect.network_settings {
            if let Some(networks_map) = network_settings.networks {
                for (network_name, network_config) in networks_map {
                    if let Some(ip_str) = network_config.ip_address {
                        if !ip_str.is_empty() {
                            if let Ok(ip_address) = IpAddr::from_str(&ip_str) {
                                let gateway = network_config
                                    .gateway
                                    .and_then(|g| if g.is_empty() { None } else { Some(g) })
                                    .and_then(|g| IpAddr::from_str(&g).ok());

                                // Try to parse subnet from IP prefix length or use a default
                                let subnet = if let Some(prefix_len) = network_config.ip_prefix_len
                                {
                                    match ip_address {
                                        IpAddr::V4(_) => {
                                            IpNetwork::new(ip_address, prefix_len as u8)
                                                .unwrap_or_else(|_| {
                                                    IpNetwork::new(ip_address, 24).unwrap()
                                                })
                                        }
                                        IpAddr::V6(_) => {
                                            IpNetwork::new(ip_address, prefix_len as u8)
                                                .unwrap_or_else(|_| {
                                                    IpNetwork::new(ip_address, 64).unwrap()
                                                })
                                        }
                                    }
                                } else {
                                    // Default subnet based on IP version
                                    match ip_address {
                                        IpAddr::V4(_) => IpNetwork::new(ip_address, 24).unwrap(),
                                        IpAddr::V6(_) => IpNetwork::new(ip_address, 64).unwrap(),
                                    }
                                };

                                networks.push(NetworkInfo {
                                    name: network_name,
                                    ip_address,
                                    gateway,
                                    subnet,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(ContainerInfo {
            id,
            name,
            labels,
            networks,
            state,
            pid,
        })
    }
}

impl DockerClient for BollardDockerClient {
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>, DockerError> {
        let options = ListContainersOptions {
            all,
            ..Default::default()
        };

        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| DockerError::ApiError(e.to_string()))?;

        let mut result = Vec::new();
        for container in containers {
            let container_info = self.convert_container_summary(container)?;
            result.push(container_info);
        }

        Ok(result)
    }

    async fn inspect_container(&self, id: &str) -> Result<ContainerInfo, DockerError> {
        let inspect_result = self
            .docker
            .inspect_container(id, Some(InspectContainerOptions::default()))
            .await
            .map_err(|e| DockerError::ApiError(e.to_string()))?;

        self.convert_container_inspect(inspect_result)
    }

    async fn get_container_networks(&self, id: &str) -> Result<Vec<NetworkInfo>, DockerError> {
        let container_info = self.inspect_container(id).await?;
        Ok(container_info.networks)
    }
}

/// Event monitor trait
pub trait EventMonitor {
    fn start_monitoring(&self) -> impl std::future::Future<Output = Result<(), EventError>> + Send;
    fn stop_monitoring(&self) -> impl std::future::Future<Output = Result<(), EventError>> + Send;
    fn subscribe_to_events(&self, handler: Box<dyn EventHandler>) -> Result<(), EventError>;
}

/// Event handler trait
pub trait EventHandler: Send + Sync {
    fn handle_container_start(
        &self,
        event: ContainerStartEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send + '_>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_container_state_equality() {
        assert_eq!(ContainerState::Running, ContainerState::Running);
        assert_ne!(ContainerState::Running, ContainerState::Stopped);
    }

    #[test]
    fn test_container_info_creation() {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());

        let container = ContainerInfo {
            id: "test-id".to_string(),
            name: "test-container".to_string(),
            labels,
            networks: vec![],
            state: ContainerState::Running,
            pid: Some(30),
        };

        assert_eq!(container.id, "test-id");
        assert_eq!(container.name, "test-container");
        assert_eq!(container.state, ContainerState::Running);
        assert_eq!(container.labels.get("app"), Some(&"test".to_string()));
        assert_eq!(container.pid, Some(30));
    }

    #[test]
    fn test_network_info_creation() {
        use std::str::FromStr;

        let ip = IpAddr::from_str("192.168.1.10").unwrap();
        let gateway = IpAddr::from_str("192.168.1.1").unwrap();
        let subnet = IpNetwork::new(ip, 24).unwrap();

        let network = NetworkInfo {
            name: "bridge".to_string(),
            ip_address: ip,
            gateway: Some(gateway),
            subnet,
        };

        assert_eq!(network.name, "bridge");
        assert_eq!(network.ip_address, ip);
        assert_eq!(network.gateway, Some(gateway));
        assert_eq!(network.subnet.prefix(), 24);
    }

    #[test]
    fn test_container_start_event_creation() {
        let container = ContainerInfo {
            id: "test-id".to_string(),
            name: "test-container".to_string(),
            labels: HashMap::new(),
            networks: vec![],
            state: ContainerState::Starting,
            pid: Some(30),
        };

        let event = ContainerStartEvent {
            container: container.clone(),
        };

        assert_eq!(event.container.id, container.id);
        assert_eq!(event.container.state, ContainerState::Starting);
    }

    // Mock implementation for testing
    pub struct MockDockerClient {
        pub containers: Vec<ContainerInfo>,
    }

    impl MockDockerClient {
        pub fn new() -> Self {
            Self {
                containers: Vec::new(),
            }
        }

        pub fn add_container(&mut self, container: ContainerInfo) {
            self.containers.push(container);
        }
    }

    impl DockerClient for MockDockerClient {
        async fn list_containers(&self, _all: bool) -> Result<Vec<ContainerInfo>, DockerError> {
            Ok(self.containers.clone())
        }

        async fn inspect_container(&self, id: &str) -> Result<ContainerInfo, DockerError> {
            self.containers
                .iter()
                .find(|c| c.id == id)
                .cloned()
                .ok_or_else(|| DockerError::ContainerNotFound {
                    container_id: id.to_string(),
                })
        }

        async fn get_container_networks(&self, id: &str) -> Result<Vec<NetworkInfo>, DockerError> {
            let container = self.inspect_container(id).await?;
            Ok(container.networks)
        }
    }

    #[tokio::test]
    async fn test_mock_docker_client() {
        let mut mock_client = MockDockerClient::new();

        let container = ContainerInfo {
            id: "test-123".to_string(),
            name: "test-container".to_string(),
            labels: HashMap::new(),
            networks: vec![],
            state: ContainerState::Running,
            pid: Some(30),
        };

        mock_client.add_container(container.clone());

        // Test list_containers
        let containers = mock_client.list_containers(false).await.unwrap();
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].id, "test-123");

        // Test inspect_container
        let inspected = mock_client.inspect_container("test-123").await.unwrap();
        assert_eq!(inspected.id, "test-123");
        assert_eq!(inspected.name, "test-container");
        assert_eq!(inspected.pid, Some(30));

        // Test container not found
        let result = mock_client.inspect_container("nonexistent").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DockerError::ContainerNotFound { container_id } => {
                assert_eq!(container_id, "nonexistent");
            }
            _ => panic!("Expected ContainerNotFound error"),
        }
    }
}
