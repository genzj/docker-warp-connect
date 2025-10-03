# Implementation Plan

- [x] 1. Set up project structure and core dependencies

  - Create dev container configuration files and start the dev container for building
  - Create Cargo.toml with required dependencies (bollard, rtnetlink, netns-rs, tokio, clap, serde, toml, thiserror, tracing)
  - Create main.rs with basic application structure
  - Set up project directory structure (src/lib.rs, src/config/, src/network/, src/docker/, src/routing/)
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 2. Implement configuration management system
- [x] 2.1 Create configuration data models and traits

  - Define AppConfig, TomlConfig, and related structs with serde derives
  - Implement ConfigurationManager trait with load_configuration method
  - Create default configuration values as constants
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 2.2 Implement TOML configuration file parsing

  - Write TOML file loading logic with error handling
  - Implement configuration validation for required fields
  - Create unit tests for TOML parsing with valid and invalid configurations
  - _Requirements: 6.3, 6.5_

- [x] 2.3 Implement environment variable configuration support

  - Add environment variable parsing with proper naming conventions
  - Implement configuration merging logic (env vars override TOML)
  - Write unit tests for environment variable parsing and precedence
  - _Requirements: 6.2, 6.4_

- [x] 2.4 Implement command-line argument parsing

  - Use clap to define CLI arguments matching configuration options
  - Implement final configuration merging (CLI > env > TOML > defaults)
  - Write integration tests for complete configuration precedence chain
  - _Requirements: 6.1, 6.4_

- [x] 3. Implement Docker API integration
- [x] 3.1 Create Docker client wrapper and container data models

  - Define ContainerInfo, NetworkInfo, and related structs
  - Implement Docker client initialization with connection handling
  - Create methods for container inspection and network discovery
  - _Requirements: 3.3, 9.1_

- [x] 3.2 Implement Docker event monitoring system

  - Create EventMonitor trait and implementation using bollard events API
  - Implement event filtering for container start/stop events
  - Add connection retry logic with exponential backoff
  - Write unit tests with mocked Docker API responses
  - _Requirements: 9.1, 9.2, 9.3, 9.4_

- [x] 3.3 Implement container classification logic

  - Create ContainerClassifier trait and implementation
  - Implement warp container detection based on name patterns
  - Implement target container detection based on labels
  - Write unit tests for container classification with various label combinations
  - _Requirements: 1.1, 2.1, 5.1, 5.2_

- [x] 4. Implement network namespace operations
- [x] 4.1 Create network namespace management wrapper

  - Implement NetworkManager trait using netns-rs
  - Create methods for entering and operating within container namespaces
  - Add error handling for insufficient privileges
  - Write unit tests with mocked namespace operations
  - _Requirements: 3.1, 8.2, 8.4_

- [x] 4.2 Implement container network discovery

  - Add methods to resolve container IP addresses across networks
  - Implement network preference logic using secondary labels
  - Handle multi-network containers with proper IP selection
  - Write unit tests for IP resolution with various network configurations
  - _Requirements: 1.4, 2.3_

- [ ] 5. Implement routing table management
- [ ] 5.1 Create route management wrapper using rtnetlink

  - Implement RouteManager trait with add/remove/list route methods
  - Define RouteEntry struct and routing rule data models
  - Add error handling for route conflicts and permission issues
  - Write unit tests with mocked netlink operations
  - _Requirements: 3.2, 8.2, 8.4_

- [ ] 5.2 Implement route calculation and application logic

  - Create methods to calculate routes from target to warp containers
  - Implement route cleanup for stopped containers
  - Add route validation and conflict detection
  - Write unit tests for route calculation with various network topologies
  - _Requirements: 1.3, 2.4_

- [ ] 6. Implement event processing and coordination
- [ ] 6.1 Create main event processing loop

  - Implement EventHandler trait that coordinates all components
  - Create event processing logic for warp container detection
  - Add logic to update all target containers when warp containers start
  - Write integration tests for warp container event processing
  - _Requirements: 1.1, 1.2, 1.3_

- [ ] 6.2 Implement target container event processing

  - Add event handling logic for target container starts
  - Implement warp container lookup and route configuration
  - Add error handling for missing warp containers
  - Write integration tests for target container event processing
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [ ] 7. Implement logging and monitoring system
- [ ] 7.1 Set up structured logging with tracing crate

  - Configure tracing subscriber with configurable log levels
  - Add logging to all major operations (container detection, route changes)
  - Implement audit logging for network modifications
  - Write tests to verify logging output for key operations
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 7.2 Add comprehensive error logging and monitoring

  - Implement error context logging with sufficient debugging information
  - Add performance metrics logging for event processing latency
  - Create health check endpoints or status reporting
  - Write tests for error logging scenarios
  - _Requirements: 4.3, 9.5_

- [ ] 8. Implement application lifecycle and service management
- [ ] 8.1 Create main application entry point and lifecycle management

  - Implement graceful shutdown handling with signal processing
  - Add resource cleanup logic for Docker connections and network operations
  - Create application state management for tracking active routes
  - Write integration tests for application startup and shutdown
  - _Requirements: 7.4, 9.5_

- [ ] 8.2 Implement service coordination and error recovery

  - Add component health monitoring and restart logic
  - Implement circuit breaker pattern for Docker API failures
  - Create recovery procedures for network operation failures
  - Write tests for error recovery scenarios
  - _Requirements: 9.4, 9.5_

- [ ] 9. Create systemd integration and deployment artifacts
- [ ] 9.1 Create systemd unit file and installation scripts

  - Write systemd service unit file with proper dependencies and restart policies
  - Create installation script for copying binary and unit file
  - Add user/group configuration for appropriate permissions
  - Document required system capabilities and permissions
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 8.1, 8.5_

- [ ] 9.2 Create configuration templates and documentation

  - Create example TOML configuration file with all options documented
  - Write README with installation, configuration, and usage instructions
  - Document required system permissions and setup procedures
  - Create troubleshooting guide for common issues
  - _Requirements: 5.5, 6.5, 8.5_

- [ ] 10. Implement comprehensive testing and validation
- [ ] 10.1 Create integration test suite with Docker containers

  - Set up test environment with real Docker containers
  - Write end-to-end tests for warp and target container scenarios
  - Test multi-network configurations and edge cases
  - Validate route table modifications in test containers
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4_

- [ ] 10.2 Add performance and security validation tests
  - Create load tests with multiple simultaneous container events
  - Test privilege validation and error handling
  - Validate configuration security and input sanitization
  - Test resource cleanup and memory leak prevention
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 9.5_
