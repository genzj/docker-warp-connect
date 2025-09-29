# Docker Network Warp

Automatic Docker container network routing manager that enables transparent traffic redirection from target containers through designated "warp containers" (proxy containers).

## Features

- **Automatic Route Management**: Monitors Docker events and dynamically updates container routing tables
- **Label-Based Configuration**: Uses Docker container labels to identify target containers and their corresponding warp containers
- **Network Namespace Operations**: Manipulates routing tables within container network namespaces using native Linux APIs
- **Multi-Network Support**: Handles containers attached to multiple networks with configurable network selection logic

## Development Setup

This project requires Linux for building and running due to native Linux kernel dependencies (netlink, network namespaces). For development on macOS or Windows, use the provided dev container configuration.

**Note**: The project will not compile on macOS or Windows due to Linux-specific dependencies (rtnetlink, netns-rs). This is expected behavior.

### Using Dev Container (Recommended)

1. Open the project in VS Code
2. Install the "Dev Containers" extension
3. Press `Ctrl+Shift+P` and select "Dev Containers: Reopen in Container"
4. The dev container will provide a Linux environment with all necessary dependencies

### Manual Linux Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install system dependencies
sudo apt-get update
sudo apt-get install -y iproute2 net-tools

# Build the project
cargo build

# Run tests
cargo test
```

## Configuration

The application supports multiple configuration sources with precedence:
CLI arguments > environment variables > TOML files > defaults

### Example TOML Configuration

See `config.toml.example` for a complete configuration example.

### Environment Variables

All configuration options can be set via environment variables with the prefix `DOCKER_NETWORK_WARP_`:

```bash
export DOCKER_NETWORK_WARP_LOG_LEVEL=debug
export DOCKER_NETWORK_WARP_DOCKER_SOCKET=/var/run/docker.sock
```

## Usage

```bash
# Run with default configuration
cargo run

# Run with custom configuration file
cargo run -- --config config.toml

# Run with custom log level
cargo run -- --log-level debug
```

## System Requirements

- Linux operating system
- Docker daemon running
- CAP_NET_ADMIN capability for routing table modifications
- Access to Docker socket

## License

MIT License - see LICENSE file for details.
