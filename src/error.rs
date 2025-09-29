//! Centralized error types and handling

use thiserror::Error;

/// Main application error type
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Docker API error: {0}")]
    Docker(#[from] DockerError),

    #[error("Network operation error: {0}")]
    Network(#[from] NetworkError),

    #[error("Route management error: {0}")]
    Route(#[from] RouteError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Event processing error: {0}")]
    Event(#[from] EventError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Docker-related errors
#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Failed to connect to Docker daemon: {0}")]
    ConnectionFailed(String),

    #[error("Container not found: {container_id}")]
    ContainerNotFound { container_id: String },

    #[error("Docker API error: {0}")]
    ApiError(String),
}

/// Network operation errors
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Failed to access network namespace: {0}")]
    NamespaceAccess(String),

    #[error("Container network not found: {container_id}")]
    NetworkNotFound { container_id: String },

    #[error("Insufficient privileges for network operation")]
    InsufficientPrivileges,

    #[error("Network operation failed: {0}")]
    OperationFailed(String),
}

/// Route management errors
#[derive(Debug, Error)]
pub enum RouteError {
    #[error("Failed to add route: {0}")]
    AddRoute(String),

    #[error("Failed to remove route: {0}")]
    RemoveRoute(String),

    #[error("Route already exists: {0}")]
    RouteExists(String),

    #[error("Invalid route configuration: {0}")]
    InvalidRoute(String),
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid configuration format: {0}")]
    InvalidFormat(String),

    #[error("Missing required configuration: {field}")]
    MissingRequired { field: String },

    #[error("Configuration validation error: {0}")]
    ValidationError(String),
}

/// Event processing errors
#[derive(Debug, Error)]
pub enum EventError {
    #[error("Failed to start event monitoring: {0}")]
    StartFailed(String),

    #[error("Event stream error: {0}")]
    StreamError(String),

    #[error("Event processing failed: {0}")]
    ProcessingFailed(String),
}

/// Event handler errors
#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Handler execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Handler configuration error: {0}")]
    ConfigurationError(String),
}