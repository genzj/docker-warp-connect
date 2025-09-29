//! Container classification logic

use std::collections::HashMap;
use crate::docker::ContainerInfo;

/// Container type classification
#[derive(Debug, Clone)]
pub enum ContainerType {
    WarpContainer(WarpContainerInfo),
    TargetContainer(TargetContainerInfo),
    Ignored,
}

/// Warp container information
#[derive(Debug, Clone)]
pub struct WarpContainerInfo {
    pub container: ContainerInfo,
    pub target_network: Option<String>,
}

/// Target container information
#[derive(Debug, Clone)]
pub struct TargetContainerInfo {
    pub container: ContainerInfo,
    pub warp_target: String,
}

/// Container classifier trait
pub trait ContainerClassifier {
    fn classify_container(&self, container: &ContainerInfo) -> ContainerType;
    fn extract_warp_target(&self, container: &ContainerInfo) -> Option<String>;
    fn extract_network_preference(&self, container: &ContainerInfo) -> Option<String>;
}

/// Default container classifier implementation
pub struct DefaultContainerClassifier {
    warp_pattern: String,
    target_label: String,
    network_preference_label: String,
}

impl DefaultContainerClassifier {
    pub fn new(warp_pattern: String, target_label: String, network_preference_label: String) -> Self {
        Self {
            warp_pattern,
            target_label,
            network_preference_label,
        }
    }
    
    fn matches_warp_pattern(&self, name: &str) -> bool {
        // Simple pattern matching - can be enhanced with regex
        if self.warp_pattern.ends_with('*') {
            let prefix = &self.warp_pattern[..self.warp_pattern.len() - 1];
            name.starts_with(prefix)
        } else {
            name == self.warp_pattern
        }
    }
}

impl ContainerClassifier for DefaultContainerClassifier {
    fn classify_container(&self, container: &ContainerInfo) -> ContainerType {
        // Check if it's a warp container by name pattern
        if self.matches_warp_pattern(&container.name) {
            let target_network = self.extract_network_preference(container);
            return ContainerType::WarpContainer(WarpContainerInfo {
                container: container.clone(),
                target_network,
            });
        }
        
        // Check if it's a target container by label
        if let Some(warp_target) = self.extract_warp_target(container) {
            return ContainerType::TargetContainer(TargetContainerInfo {
                container: container.clone(),
                warp_target,
            });
        }
        
        ContainerType::Ignored
    }
    
    fn extract_warp_target(&self, container: &ContainerInfo) -> Option<String> {
        container.labels.get(&self.target_label).cloned()
    }
    
    fn extract_network_preference(&self, container: &ContainerInfo) -> Option<String> {
        container.labels.get(&self.network_preference_label).cloned()
    }
}