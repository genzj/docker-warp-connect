# Product Overview

Docker Network Warp is a Rust-based system service that automatically manages container network routing by monitoring Docker events and manipulating network namespaces. The system enables transparent traffic redirection from target containers through designated "warp containers" (proxy containers) without manual intervention.

## Core Functionality

- **Automatic Route Management**: Monitors Docker events in real-time and dynamically updates container routing tables
- **Label-Based Configuration**: Uses Docker container labels to identify target containers and their corresponding warp containers
- **Network Namespace Operations**: Manipulates routing tables within container network namespaces using native Linux APIs
- **Multi-Network Support**: Handles containers attached to multiple networks with configurable network selection logic

## Key Use Cases

- Transparent proxy routing for containerized applications
- Network policy enforcement through designated proxy containers
- Dynamic traffic redirection based on container lifecycle events
- Automated network configuration for microservices architectures

## Target Users

System administrators and DevOps engineers managing containerized environments who need automated network routing policies and transparent proxy configurations.
