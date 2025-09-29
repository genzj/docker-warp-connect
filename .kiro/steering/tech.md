# Technology Stack

## Language & Runtime

- **Rust**: Primary language for system-level performance and memory safety
- **Tokio**: Async runtime for event-driven architecture and Docker API streaming

## Core Dependencies

- **bollard**: Docker API client for container monitoring and inspection
- **rtnetlink**: Native Linux netlink interface for routing table manipulation
- **netns-rs**: Network namespace operations without shell dependencies
- **clap**: Command-line argument parsing with structured configuration
- **serde + toml**: Configuration serialization and TOML file parsing
- **thiserror**: Structured error handling and propagation
- **tracing**: Structured logging and observability

## Architecture Patterns

- **Event-Driven**: Reactive architecture responding to Docker container lifecycle events
- **Component-Based**: Modular design with clear separation of concerns (EventMonitor, RouteManager, NetworkManager, etc.)
- **Native APIs Only**: No shell command execution - all operations through Rust crates and system APIs
- **Multi-Source Configuration**: Layered configuration with precedence (CLI > env vars > TOML > defaults)

## Build System

**Linux-Only**: This project is designed exclusively for Linux systems and should only be built on Linux. Not supported on macOS or Windows due to native Linux kernel dependencies (netlink, network namespaces). For development on macOS or Windows, use the provided dev container configuration which provides a Linux environment with all necessary dependencies and kernel features for building and testing.

```bash
# Development
cargo build
cargo test
cargo clippy
cargo fmt

# Release build
cargo build --release

# Run with configuration
cargo run -- --config config.toml

# Install systemd service
sudo cp target/release/docker-network-warp /usr/local/bin/
sudo cp docker-network-warp.service /etc/systemd/system/
sudo systemctl enable docker-network-warp
sudo systemctl start docker-network-warp
```

## System Integration

- **Systemd Service**: Native Linux service integration with proper lifecycle management
- **Network Capabilities**: Requires CAP_NET_ADMIN for routing table modifications
- **Docker Socket**: Connects to Docker daemon via Unix socket or TCP
- **Configuration**: TOML files, environment variables, and CLI arguments
