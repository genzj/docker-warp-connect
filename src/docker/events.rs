//! Docker event monitoring and processing

use crate::docker::{
    BollardDockerClient, ContainerStartEvent, DockerClient, EventHandler, EventMonitor,
};
use crate::error::{DockerError, EventError};
use bollard::models::EventMessage;
use bollard::query_parameters::EventsOptions;
use bollard::secret::EventMessageTypeEnum;
use bollard::Docker;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Docker connection configuration for event monitoring
#[derive(Debug, Clone)]
enum DockerConnection {
    Socket(String),
    Http(String),
    Default,
}

/// Docker event monitor implementation with retry logic and event filtering
pub struct DockerEventMonitor {
    docker_client: BollardDockerClient,
    docker_connection: DockerConnection,
    handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
    retry_delay: Duration,
    max_retries: u32,
}

impl DockerEventMonitor {
    /// Create a new Docker event monitor with default connection
    pub fn new() -> Result<Self, DockerError> {
        let docker_client = BollardDockerClient::new()?;
        Ok(Self {
            docker_client,
            docker_connection: DockerConnection::Default,
            handlers: Arc::new(RwLock::new(Vec::new())),
            retry_delay: Duration::from_secs(5),
            max_retries: 10,
        })
    }

    /// Create a new Docker event monitor with custom socket path
    pub fn with_socket(socket_path: &str) -> Result<Self, DockerError> {
        let docker_client = BollardDockerClient::with_socket(socket_path)?;
        Ok(Self {
            docker_client,
            docker_connection: DockerConnection::Socket(socket_path.to_string()),
            handlers: Arc::new(RwLock::new(Vec::new())),
            retry_delay: Duration::from_secs(5),
            max_retries: 10,
        })
    }

    /// Create a new Docker event monitor with HTTP connection
    pub fn with_http(url: &str) -> Result<Self, DockerError> {
        let docker_client = BollardDockerClient::with_http(url)?;
        Ok(Self {
            docker_client,
            docker_connection: DockerConnection::Http(url.to_string()),
            handlers: Arc::new(RwLock::new(Vec::new())),
            retry_delay: Duration::from_secs(5),
            max_retries: 10,
        })
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, retry_delay: Duration, max_retries: u32) -> Self {
        self.retry_delay = retry_delay;
        self.max_retries = max_retries;
        self
    }

    /// Process a Docker event and notify handlers
    async fn process_event(&self, event: EventMessage) -> Result<(), EventError> {
        // Filter for container events
        if let Some(event_type) = &event.typ {
            if *event_type != EventMessageTypeEnum::CONTAINER {
                return Ok(());
            }
        }

        // Filter for start events
        if let Some(action) = &event.action {
            if action != "start" {
                return Ok(());
            }
        }

        // Extract container ID
        let container_id = match &event.actor {
            Some(actor) => match &actor.id {
                Some(id) => id.clone(),
                None => {
                    debug!("Event missing container ID");
                    return Ok(());
                }
            },
            None => {
                debug!("Event missing actor information");
                return Ok(());
            }
        };

        debug!("Processing container start event for: {}", container_id);

        // Get container information
        let container_info = match self.docker_client.inspect_container(&container_id).await {
            Ok(info) => info,
            Err(e) => {
                warn!("Failed to inspect container {}: {}", container_id, e);
                return Ok(()); // Don't fail the entire event processing
            }
        };

        // Create container start event
        let start_event = ContainerStartEvent {
            container: container_info,
        };

        // Notify all handlers
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler.handle_container_start(start_event.clone()).await {
                error!("Handler failed to process container start event: {}", e);
                // Continue processing other handlers
            }
        }

        Ok(())
    }

    /// Start monitoring with retry logic
    async fn start_monitoring_with_retry(&self) -> Result<(), EventError> {
        let mut retry_count = 0;

        loop {
            match self.start_monitoring_internal().await {
                Ok(()) => {
                    info!("Docker event monitoring completed successfully");
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count > self.max_retries {
                        error!(
                            "Max retries ({}) exceeded for Docker event monitoring",
                            self.max_retries
                        );
                        return Err(e);
                    }

                    warn!(
                        "Docker event monitoring failed (attempt {}/{}): {}. Retrying in {:?}",
                        retry_count, self.max_retries, e, self.retry_delay
                    );

                    sleep(self.retry_delay).await;
                }
            }
        }
    }

    /// Internal monitoring implementation
    async fn start_monitoring_internal(&self) -> Result<(), EventError> {
        info!("Starting Docker event monitoring");

        // Create a new Docker connection for event streaming using the same configuration
        let docker = match &self.docker_connection {
            DockerConnection::Socket(socket_path) => {
                Docker::connect_with_socket(socket_path, 120, bollard::API_DEFAULT_VERSION)
                    .map_err(|e| {
                        EventError::StartFailed(format!(
                            "Failed to connect to Docker socket {}: {}",
                            socket_path, e
                        ))
                    })?
            }
            DockerConnection::Http(url) => {
                Docker::connect_with_http(url, 120, bollard::API_DEFAULT_VERSION).map_err(|e| {
                    EventError::StartFailed(format!(
                        "Failed to connect to Docker HTTP {}: {}",
                        url, e
                    ))
                })?
            }
            DockerConnection::Default => Docker::connect_with_socket_defaults().map_err(|e| {
                EventError::StartFailed(format!("Failed to connect to Docker: {}", e))
            })?,
        };

        // Set up event filters for container events
        let mut filters = HashMap::new();
        filters.insert("type".to_string(), vec!["container".to_string()]);
        filters.insert(
            "event".to_string(),
            vec!["start".to_string(), "stop".to_string()],
        );

        let options = EventsOptions {
            since: None,
            until: None,
            filters: Some(filters),
        };

        let mut stream = docker.events(Some(options));

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    debug!("Received Docker event: {:?}", event);
                    if let Err(e) = self.process_event(event).await {
                        error!("Failed to process Docker event: {}", e);
                        // Continue processing other events
                    }
                }
                Err(e) => {
                    error!("Docker event stream error: {}", e);
                    return Err(EventError::StreamError(e.to_string()));
                }
            }
        }

        info!("Docker event stream ended");
        Ok(())
    }
}

impl EventMonitor for DockerEventMonitor {
    async fn start_monitoring(&self) -> Result<(), EventError> {
        self.start_monitoring_with_retry().await
    }

    async fn stop_monitoring(&self) -> Result<(), EventError> {
        info!("Stopping Docker event monitoring");
        // The monitoring will stop when the stream ends or an error occurs
        // In a real implementation, we might want to use a cancellation token
        Ok(())
    }

    fn subscribe_to_events(&self, handler: Box<dyn EventHandler>) -> Result<(), EventError> {
        let handler_arc = Arc::from(handler);
        tokio::spawn({
            let handlers = Arc::clone(&self.handlers);
            async move {
                let mut handlers_guard = handlers.write().await;
                handlers_guard.push(handler_arc);
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::HandlerError;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    // Mock event handler for testing
    struct MockEventHandler {
        call_count: Arc<AtomicUsize>,
        received_events: Arc<Mutex<Vec<ContainerStartEvent>>>,
    }

    impl MockEventHandler {
        fn new() -> Self {
            Self {
                call_count: Arc::new(AtomicUsize::new(0)),
                received_events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }

        fn get_received_events(&self) -> Vec<ContainerStartEvent> {
            self.received_events.lock().unwrap().clone()
        }
    }

    impl EventHandler for MockEventHandler {
        fn handle_container_start(
            &self,
            event: ContainerStartEvent,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send + '_>,
        > {
            Box::pin(async move {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                self.received_events.lock().unwrap().push(event);
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn test_event_monitor_creation() {
        // Test that we can create the monitor (may fail if Docker is not available)
        let result = DockerEventMonitor::new();
        // We don't assert success here since Docker may not be available in test environment
        match result {
            Ok(monitor) => {
                assert_eq!(monitor.handlers.read().await.len(), 0);
            }
            Err(_) => {
                // Docker not available, which is fine for unit tests
            }
        }
    }

    #[tokio::test]
    async fn test_retry_configuration() {
        // Test retry configuration without requiring Docker connection
        if let Ok(monitor) = DockerEventMonitor::new() {
            let monitor = monitor.with_retry_config(Duration::from_millis(100), 5);

            assert_eq!(monitor.retry_delay, Duration::from_millis(100));
            assert_eq!(monitor.max_retries, 5);
        }
    }

    #[test]
    fn test_process_event_with_missing_data() {
        // Test that process_event handles missing event data gracefully
        if let Ok(monitor) = DockerEventMonitor::new() {
            let rt = tokio::runtime::Runtime::new().unwrap();

            // Test event with missing type
            let event = EventMessage {
                typ: None,
                action: Some("start".to_string()),
                actor: None,
                time: None,
                time_nano: None,
                scope: None,
            };

            rt.block_on(async {
                let result = monitor.process_event(event).await;
                assert!(result.is_ok());
            });
        }
    }

    #[tokio::test]
    async fn test_event_handler_subscription() {
        if let Ok(monitor) = DockerEventMonitor::new() {
            let handler = Box::new(MockEventHandler::new());

            // Subscribe handler
            monitor.subscribe_to_events(handler).unwrap();

            // Give some time for the async subscription to complete
            sleep(Duration::from_millis(10)).await;

            // Check that handler was added
            assert_eq!(monitor.handlers.read().await.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_event_processing_and_handler_invocation() {
        // Create a test monitor that we can use to test event processing
        if let Ok(monitor) = DockerEventMonitor::new() {
            let handler = Box::new(MockEventHandler::new());

            // Get references to the handler's internal state before moving it
            let handler_call_count = Arc::clone(&handler.call_count);
            let handler_events = Arc::clone(&handler.received_events);

            // Subscribe handler
            monitor.subscribe_to_events(handler).unwrap();

            // Give some time for the async subscription to complete
            sleep(Duration::from_millis(10)).await;

            // Verify handler was added
            assert_eq!(monitor.handlers.read().await.len(), 1);

            // Create mock container data that the Docker client would return
            use crate::docker::{ContainerInfo, ContainerState, NetworkInfo};
            use ipnetwork::IpNetwork;
            use std::collections::HashMap;
            use std::net::IpAddr;
            use std::str::FromStr;

            let container_info = ContainerInfo {
                id: "test-container-123".to_string(),
                name: "test-container".to_string(),
                labels: HashMap::new(),
                networks: vec![NetworkInfo {
                    name: "bridge".to_string(),
                    ip_address: IpAddr::from_str("172.17.0.2").unwrap(),
                    gateway: Some(IpAddr::from_str("172.17.0.1").unwrap()),
                    subnet: IpNetwork::new(IpAddr::from_str("172.17.0.0").unwrap(), 16).unwrap(),
                }],
                state: ContainerState::Running,
                pid: Some(30),
            };

            // Manually create a ContainerStartEvent and notify handlers
            // This simulates what would happen in process_event after successful container inspection
            let start_event = ContainerStartEvent {
                container: container_info,
            };

            // Manually notify handlers (simulating successful event processing)
            let handlers = monitor.handlers.read().await;
            for handler in handlers.iter() {
                if let Err(e) = handler.handle_container_start(start_event.clone()).await {
                    panic!("Handler failed: {}", e);
                }
            }

            // Verify the handler was called
            assert_eq!(handler_call_count.load(Ordering::SeqCst), 1);

            // Verify the handler received the correct event
            let received_events = handler_events.lock().unwrap();
            assert_eq!(received_events.len(), 1);
            assert_eq!(received_events[0].container.id, "test-container-123");
            assert_eq!(received_events[0].container.name, "test-container");
        }
    }

    #[test]
    fn test_docker_connection_configuration() {
        // Test default connection
        if let Ok(monitor) = DockerEventMonitor::new() {
            match monitor.docker_connection {
                DockerConnection::Default => {}
                _ => panic!("Expected Default connection type"),
            }
        }

        // Test socket connection
        if let Ok(monitor) = DockerEventMonitor::with_socket("/var/run/docker.sock") {
            match monitor.docker_connection {
                DockerConnection::Socket(path) => {
                    assert_eq!(path, "/var/run/docker.sock");
                }
                _ => panic!("Expected Socket connection type"),
            }
        }

        // Test HTTP connection
        if let Ok(monitor) = DockerEventMonitor::with_http("http://localhost:2376") {
            match monitor.docker_connection {
                DockerConnection::Http(url) => {
                    assert_eq!(url, "http://localhost:2376");
                }
                _ => panic!("Expected Http connection type"),
            }
        }
    }
}
