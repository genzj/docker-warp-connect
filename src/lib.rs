//! Docker Network Warp - Automatic container network routing manager
//! 
//! This library provides components for monitoring Docker events and automatically
//! configuring container network routes to direct traffic through designated warp containers.

pub mod config;
pub mod docker;
pub mod network;
pub mod routing;
pub mod error;

pub use error::AppError;