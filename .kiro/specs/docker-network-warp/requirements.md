# Requirements Document

## Introduction

This project implements a Docker network routing manager that automatically configures container network routes to direct traffic through designated "warp containers". The system monitors Docker events in real-time and dynamically updates container routing tables using native Rust APIs, enabling transparent traffic redirection for containers with specific labels through designated proxy containers.

## Requirements

### Requirement 1

**User Story:** As a system administrator, I want the application to automatically detect and configure warp containers when they start, so that target containers can route traffic through them without manual intervention.

#### Acceptance Criteria

1. WHEN a Docker container starts with a specified name pattern THEN the system SHALL recognize it as a warp container
2. WHEN a warp container is detected THEN the system SHALL iterate through all running containers with target labels and the labels' values match the warp container's name
3. WHEN updating target container routes THEN the system SHALL add routing entries directing specified traffic to the warp container

### Requirement 2

**User Story:** As a system administrator, I want target containers to automatically route traffic through warp containers when they start, so that network policies are applied consistently.

#### Acceptance Criteria

1. WHEN a Docker container starts with a specified target label THEN the system SHALL recognize it as a target container
2. WHEN a target container is detected THEN the system SHALL search for the corresponding warp container using the label value
3. IF a matching warp container is found THEN the system SHALL configure the target container's routing table

### Requirement 2.5

**User Story:** As a system administrator, I want the system to find next hop's ip address automatically by inspecting warp container's network configuration.

#### Acceptance Criteria

1. WHEN configuring routes THEN the system SHALL set the warp container's IP as the next-hop for specified traffic rules
2. WHEN adding routing entries THEN the system SHALL inspect the warp container's network configuration to confirm its IP address to be used as the next hop
3. IF a warp container joins multiple networks THEN the system SHALL use a secondary label of the warp container to determine which network to use as next-hop, OTHERWISE an error should be reported and the updating should be aborted.

### Requirement 3

**User Story:** As a system administrator, I want the application to use native Rust APIs for network operations, so that it works reliably across different platforms without shell dependencies.

#### Acceptance Criteria

1. WHEN performing network namespace operations THEN the system SHALL use the netns-rs crate or equivalent
2. WHEN manipulating routing tables THEN the system SHALL use the rtnetlink crate or equivalent
3. WHEN interacting with Docker THEN the system SHALL use the docker-api crate or equivalent
4. WHEN performing system operations THEN the system SHALL NOT execute shell commands

### Requirement 4

**User Story:** As a system administrator, I want comprehensive logging and monitoring capabilities, so that I can debug issues and audit network changes.

#### Acceptance Criteria

1. WHEN the application performs any network operation THEN it SHALL log the action with appropriate detail level
2. WHEN Docker events are received THEN the system SHALL log event details for audit purposes
3. WHEN errors occur THEN the system SHALL log error details with sufficient context for debugging
4. WHEN routing tables are modified THEN the system SHALL log the specific changes made
5. WHEN containers are detected THEN the system SHALL log container identification and classification

### Requirement 5

**User Story:** As a system administrator, I want flexible configuration options, so that I can adapt the system to different environments and requirements.

#### Acceptance Criteria

1. WHEN configuring label names THEN the system SHALL support custom label configurations with sensible defaults
2. WHEN specifying warp container identification THEN the system SHALL support configurable name patterns
3. WHEN defining routing rules THEN the system SHALL support configurable traffic matching criteria by specifying a CIDR
4. WHEN setting network preferences THEN the system SHALL support configurable network selection logic by specifying a second label whose value selects one network that the warp container is attached to
5. WHEN running in different environments THEN all label names and patterns SHALL be configurable

### Requirement 6

**User Story:** As a system administrator, I want multiple configuration methods with proper precedence, so that I can manage settings flexibly across different deployment scenarios.

#### Acceptance Criteria

1. WHEN configuration is provided via command line arguments THEN they SHALL take highest precedence
2. WHEN configuration is provided via environment variables THEN they SHALL override TOML file settings
3. WHEN configuration is provided via TOML files THEN they SHALL provide base configuration values
4. IF the same setting is specified in multiple sources THEN command line SHALL override environment variables SHALL override TOML files
5. WHEN no configuration is provided for a setting THEN the system SHALL use documented default values

### Requirement 7

**User Story:** As a system administrator, I want systemd integration, so that the application can run as a system service with proper lifecycle management.

#### Acceptance Criteria

1. WHEN installing the application THEN it SHALL provide a systemd unit file
2. WHEN the systemd service starts THEN it SHALL automatically start the Docker routing manager
3. WHEN the system boots THEN the service SHALL start automatically if enabled
4. WHEN the service fails THEN systemd SHALL handle restart policies according to configuration
5. WHEN stopping the service THEN it SHALL gracefully shut down and clean up resources

### Requirement 8

**User Story:** As a security-conscious administrator, I want the application to operate with explicitly granted privileges, so that it doesn't escalate permissions or introduce security vulnerabilities.

#### Acceptance Criteria

1. WHEN the application starts THEN it SHALL NOT attempt to escalate privileges using sudo or similar mechanisms
2. WHEN network operations require elevated privileges THEN the system SHALL fail gracefully with clear error messages if insufficient permissions
3. WHEN accessing Docker APIs THEN the system SHALL use the permissions of the executing user
4. WHEN manipulating network namespaces THEN the system SHALL require appropriate capabilities to be granted externally
5. IF insufficient privileges are detected THEN the system SHALL provide clear guidance on required permissions

### Requirement 9

**User Story:** As a system administrator, I want real-time Docker event monitoring, so that network changes are applied immediately when containers start or stop.

#### Acceptance Criteria

1. WHEN the application starts THEN it SHALL establish a connection to the Docker events API
2. WHEN Docker containers start THEN the system SHALL receive and process start events within seconds
3. IF the Docker connection is lost THEN the system SHALL attempt to reconnect automatically
4. WHEN processing events THEN the system SHALL handle event processing errors without crashing
