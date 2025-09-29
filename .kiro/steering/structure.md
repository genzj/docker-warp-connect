# Project Structure

## Directory Organization

```
├── src/
│   ├── main.rs              # Application entry point and CLI setup
│   ├── lib.rs               # Library root with public API exports
│   ├── config/              # Configuration management
│   │   ├── mod.rs           # Configuration traits and main logic
│   │   ├── toml.rs          # TOML file parsing
│   │   ├── env.rs           # Environment variable handling
│   │   └── cli.rs           # Command-line argument parsing
│   ├── docker/              # Docker API integration
│   │   ├── mod.rs           # Docker client wrapper and traits
│   │   ├── events.rs        # Event monitoring and processing
│   │   └── classifier.rs    # Container classification logic
│   ├── network/             # Network operations
│   │   ├── mod.rs           # Network manager traits and core logic
│   │   ├── namespace.rs     # Network namespace operations
│   │   └── discovery.rs     # Container network discovery
│   ├── routing/             # Routing table management
│   │   ├── mod.rs           # Route manager traits and core logic
│   │   ├── manager.rs       # Route manipulation using rtnetlink
│   │   └── rules.rs         # Routing rule calculation and validation
│   └── error.rs             # Centralized error types and handling
├── tests/                   # Integration tests
├── examples/                # Example configurations and usage
├── docker-network-warp.service  # Systemd unit file
├── config.toml.example      # Example configuration file
└── Cargo.toml              # Dependencies and project metadata
```

## Code Organization Principles

- **Trait-Based Design**: Each major component (EventMonitor, RouteManager, NetworkManager, ConfigurationManager) defined as traits for testability
- **Error Propagation**: Centralized error types in `error.rs` using `thiserror` for structured error handling
- **Async Throughout**: All I/O operations use async/await with Tokio runtime
- **No Shell Dependencies**: All system operations through native Rust crates and APIs

## Key Files and Responsibilities

- **main.rs**: CLI setup, configuration loading, component initialization, and application lifecycle
- **config/**: Multi-source configuration with precedence handling (CLI > env > TOML > defaults)
- **docker/events.rs**: Docker event stream processing and container lifecycle monitoring
- **docker/classifier.rs**: Label-based container type detection (warp vs target containers)
- **network/namespace.rs**: Network namespace operations using netns-rs
- **routing/manager.rs**: Routing table manipulation using rtnetlink
- **error.rs**: Comprehensive error types for all failure modes

## Testing Structure

- **Unit Tests**: Alongside source files using `#[cfg(test)]` modules
- **Integration Tests**: In `tests/` directory with real Docker containers
- **Mocking**: Trait-based design enables easy mocking of external dependencies
