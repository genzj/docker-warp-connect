//! Container classification logic

use crate::docker::ContainerInfo;
use regex::Regex;

/// Container type classification
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerType {
    WarpContainer(WarpContainerInfo),
    TargetContainer(TargetContainerInfo),
    Ignored,
}

/// Warp container information
#[derive(Debug, Clone, PartialEq)]
pub struct WarpContainerInfo {
    pub container: ContainerInfo,
    pub target_network: Option<String>,
}

/// Target container information
#[derive(Debug, Clone, PartialEq)]
pub struct TargetContainerInfo {
    pub container: ContainerInfo,
    pub warp_target: String,
}

/// Container classifier trait
pub trait ContainerClassifier {
    /// Classify a container based on its metadata
    fn classify_container(&self, container: &ContainerInfo) -> ContainerType;

    /// Extract warp target from container labels
    fn extract_warp_target(&self, container: &ContainerInfo) -> Option<String>;

    /// Extract network preference from container labels
    fn extract_network_preference(&self, container: &ContainerInfo) -> Option<String>;

    /// Check if a container name matches the warp pattern
    fn is_warp_container(&self, container: &ContainerInfo) -> bool;

    /// Check if a container has target labels
    fn is_target_container(&self, container: &ContainerInfo) -> bool;
}

/// Default container classifier implementation with regex support
pub struct DefaultContainerClassifier {
    warp_raw_pattern: String,
    warp_regex: Option<Regex>,
    target_label: String,
    network_preference_label: String,
}

impl DefaultContainerClassifier {
    /// Create a new classifier with the given configuration
    pub fn new(
        warp_pattern: String,
        target_label: String,
        network_preference_label: String,
    ) -> Result<Self, regex::Error> {
        // Try to compile as regex if it contains regex metacharacters
        let warp_regex = if warp_pattern.contains([
            '*', '+', '?', '^', '$', '[', ']', '(', ')', '{', '}', '|', '\\',
        ]) {
            Some(Regex::new(&warp_pattern)?)
        } else {
            None
        };

        Ok(Self {
            warp_raw_pattern: warp_pattern,
            warp_regex,
            target_label,
            network_preference_label,
        })
    }

    /// Create a classifier with simple pattern matching (no regex)
    pub fn with_simple_pattern(
        warp_pattern: String,
        target_label: String,
        network_preference_label: String,
    ) -> Self {
        Self {
            warp_raw_pattern: warp_pattern,
            warp_regex: None,
            target_label,
            network_preference_label,
        }
    }

    /// Check if a name matches the warp pattern
    fn matches_warp_pattern(&self, name: &str) -> bool {
        if let Some(regex) = &self.warp_regex {
            regex.is_match(name)
        } else {
            // Simple pattern matching with wildcard support
            self.matches_simple_pattern(name)
        }
    }

    /// Match a name against a simple pattern with wildcard support
    fn matches_simple_pattern(&self, name: &str) -> bool {
        if self.warp_raw_pattern.ends_with('*') {
            let prefix = &self.warp_raw_pattern[..self.warp_raw_pattern.len() - 1];
            name.starts_with(prefix)
        } else if self.warp_raw_pattern.starts_with('*') {
            let suffix = &self.warp_raw_pattern[1..];
            name.ends_with(suffix)
        } else if self.warp_raw_pattern.contains('*') {
            // Handle patterns like "prefix*suffix"
            let parts: Vec<&str> = self.warp_raw_pattern.split('*').collect();
            if parts.len() == 2 {
                name.starts_with(parts[0]) && name.ends_with(parts[1])
            } else {
                name == self.warp_raw_pattern
            }
        } else {
            name == self.warp_raw_pattern
        }
    }

    /// Validate that a warp container has a valid network configuration
    fn validate_warp_container(&self, container: &ContainerInfo) -> bool {
        // If network preference is specified, ensure the container is on that network
        if let Some(preferred_network) = self.extract_network_preference(container) {
            container
                .networks
                .iter()
                .any(|net| net.name == preferred_network)
        } else {
            // If no preference, container must have at least one network
            !container.networks.is_empty()
        }
    }

    /// Validate that a target container configuration is valid
    fn validate_target_container(&self, container: &ContainerInfo) -> bool {
        // Target container must have the target label and at least one network
        self.extract_warp_target(container).is_some() && !container.networks.is_empty()
    }
}

impl ContainerClassifier for DefaultContainerClassifier {
    fn classify_container(&self, container: &ContainerInfo) -> ContainerType {
        // Check if it's a warp container by name pattern
        if self.is_warp_container(container) {
            if self.validate_warp_container(container) {
                let target_network = self.extract_network_preference(container);
                return ContainerType::WarpContainer(WarpContainerInfo {
                    container: container.clone(),
                    target_network,
                });
            }
        }

        // Check if it's a target container by label
        if self.is_target_container(container) {
            if self.validate_target_container(container) {
                if let Some(warp_target) = self.extract_warp_target(container) {
                    return ContainerType::TargetContainer(TargetContainerInfo {
                        container: container.clone(),
                        warp_target,
                    });
                }
            }
        }

        ContainerType::Ignored
    }

    fn extract_warp_target(&self, container: &ContainerInfo) -> Option<String> {
        container.labels.get(&self.target_label).cloned()
    }

    fn extract_network_preference(&self, container: &ContainerInfo) -> Option<String> {
        container
            .labels
            .get(&self.network_preference_label)
            .cloned()
    }

    fn is_warp_container(&self, container: &ContainerInfo) -> bool {
        self.matches_warp_pattern(&container.name)
    }

    fn is_target_container(&self, container: &ContainerInfo) -> bool {
        container.labels.contains_key(&self.target_label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::{ContainerState, NetworkInfo};
    use ipnetwork::IpNetwork;
    use std::collections::HashMap;
    use std::net::IpAddr;
    use std::str::FromStr;

    fn create_test_container(
        name: &str,
        labels: HashMap<String, String>,
        networks: Vec<NetworkInfo>,
    ) -> ContainerInfo {
        ContainerInfo {
            id: format!("test-{}", name),
            name: name.to_string(),
            labels,
            networks,
            state: ContainerState::Running,
            pid: Some(30),
        }
    }

    fn create_test_network(name: &str, ip: &str) -> NetworkInfo {
        let ip_addr = IpAddr::from_str(ip).unwrap();
        NetworkInfo {
            name: name.to_string(),
            ip_address: ip_addr,
            gateway: None,
            subnet: IpNetwork::new(ip_addr, 24).unwrap(),
        }
    }

    #[test]
    fn test_simple_warp_pattern_matching() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-proxy".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Exact match
        let container = create_test_container(
            "warp-proxy",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );
        assert!(classifier.is_warp_container(&container));

        // No match
        let container = create_test_container(
            "other-container",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.3")],
        );
        assert!(!classifier.is_warp_container(&container));
    }

    #[test]
    fn test_wildcard_warp_pattern_matching() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Prefix match
        let container = create_test_container(
            "warp-proxy-1",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );
        assert!(classifier.is_warp_container(&container));

        let container = create_test_container(
            "warp-gateway",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.3")],
        );
        assert!(classifier.is_warp_container(&container));

        // No match
        let container = create_test_container(
            "proxy-warp",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.4")],
        );
        assert!(!classifier.is_warp_container(&container));
    }

    #[test]
    fn test_suffix_wildcard_pattern() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "*-proxy".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Suffix match
        let container = create_test_container(
            "warp-proxy",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );
        assert!(classifier.is_warp_container(&container));

        let container = create_test_container(
            "http-proxy",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.3")],
        );
        assert!(classifier.is_warp_container(&container));

        // No match
        let container = create_test_container(
            "proxy-server",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.4")],
        );
        assert!(!classifier.is_warp_container(&container));
    }

    #[test]
    fn test_regex_pattern_matching() {
        let classifier = DefaultContainerClassifier::new(
            r"^warp-\d+$".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        )
        .unwrap();

        // Regex match
        let container = create_test_container(
            "warp-123",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );
        assert!(classifier.is_warp_container(&container));

        let container = create_test_container(
            "warp-456",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.3")],
        );
        assert!(classifier.is_warp_container(&container));

        // No match
        let container = create_test_container(
            "warp-abc",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.4")],
        );
        assert!(!classifier.is_warp_container(&container));

        let container = create_test_container(
            "proxy-warp-123",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.5")],
        );
        assert!(!classifier.is_warp_container(&container));
    }

    #[test]
    fn test_target_container_detection() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Target container with label
        let mut labels = HashMap::new();
        labels.insert("warp.target".to_string(), "warp-proxy-1".to_string());
        let container = create_test_container(
            "app-server",
            labels,
            vec![create_test_network("bridge", "172.17.0.2")],
        );
        assert!(classifier.is_target_container(&container));

        // Container without target label
        let container = create_test_container(
            "other-server",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.3")],
        );
        assert!(!classifier.is_target_container(&container));
    }

    #[test]
    fn test_warp_container_classification() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Valid warp container
        let container = create_test_container(
            "warp-proxy-1",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );

        match classifier.classify_container(&container) {
            ContainerType::WarpContainer(info) => {
                assert_eq!(info.container.name, "warp-proxy-1");
                assert_eq!(info.target_network, None);
            }
            _ => panic!("Expected WarpContainer classification"),
        }
    }

    #[test]
    fn test_warp_container_with_network_preference() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Warp container with network preference
        let mut labels = HashMap::new();
        labels.insert("warp.network".to_string(), "custom-network".to_string());
        let container = create_test_container(
            "warp-proxy-1",
            labels,
            vec![
                create_test_network("bridge", "172.17.0.2"),
                create_test_network("custom-network", "10.0.0.2"),
            ],
        );

        match classifier.classify_container(&container) {
            ContainerType::WarpContainer(info) => {
                assert_eq!(info.container.name, "warp-proxy-1");
                assert_eq!(info.target_network, Some("custom-network".to_string()));
            }
            _ => panic!("Expected WarpContainer classification"),
        }
    }

    #[test]
    fn test_target_container_classification() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Valid target container
        let mut labels = HashMap::new();
        labels.insert("warp.target".to_string(), "warp-proxy-1".to_string());
        let container = create_test_container(
            "app-server",
            labels,
            vec![create_test_network("bridge", "172.17.0.2")],
        );

        match classifier.classify_container(&container) {
            ContainerType::TargetContainer(info) => {
                assert_eq!(info.container.name, "app-server");
                assert_eq!(info.warp_target, "warp-proxy-1");
            }
            _ => panic!("Expected TargetContainer classification"),
        }
    }

    #[test]
    fn test_ignored_container_classification() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Container that doesn't match any pattern
        let container = create_test_container(
            "regular-app",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );

        match classifier.classify_container(&container) {
            ContainerType::Ignored => {}
            _ => panic!("Expected Ignored classification"),
        }
    }

    #[test]
    fn test_warp_container_validation() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Warp container without networks should be ignored
        let container = create_test_container("warp-proxy-1", HashMap::new(), vec![]);
        match classifier.classify_container(&container) {
            ContainerType::Ignored => {}
            _ => panic!("Expected Ignored classification for warp container without networks"),
        }

        // Warp container with network preference but missing that network should be ignored
        let mut labels = HashMap::new();
        labels.insert("warp.network".to_string(), "missing-network".to_string());
        let container = create_test_container(
            "warp-proxy-1",
            labels,
            vec![create_test_network("bridge", "172.17.0.2")],
        );
        match classifier.classify_container(&container) {
            ContainerType::Ignored => {}
            _ => panic!(
                "Expected Ignored classification for warp container with missing preferred network"
            ),
        }
    }

    #[test]
    fn test_target_container_validation() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        // Target container without networks should be ignored
        let mut labels = HashMap::new();
        labels.insert("warp.target".to_string(), "warp-proxy-1".to_string());
        let container = create_test_container("app-server", labels, vec![]);
        match classifier.classify_container(&container) {
            ContainerType::Ignored => {}
            _ => panic!("Expected Ignored classification for target container without networks"),
        }
    }

    #[test]
    fn test_extract_methods() {
        let classifier = DefaultContainerClassifier::with_simple_pattern(
            "warp-*".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );

        let mut labels = HashMap::new();
        labels.insert("warp.target".to_string(), "warp-proxy-1".to_string());
        labels.insert("warp.network".to_string(), "custom-network".to_string());
        labels.insert("other.label".to_string(), "other-value".to_string());

        let container = create_test_container(
            "test-container",
            labels,
            vec![create_test_network("bridge", "172.17.0.2")],
        );

        assert_eq!(
            classifier.extract_warp_target(&container),
            Some("warp-proxy-1".to_string())
        );
        assert_eq!(
            classifier.extract_network_preference(&container),
            Some("custom-network".to_string())
        );

        // Test with container without labels
        let empty_container = create_test_container(
            "empty-container",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.3")],
        );

        assert_eq!(classifier.extract_warp_target(&empty_container), None);
        assert_eq!(
            classifier.extract_network_preference(&empty_container),
            None
        );
    }

    #[test]
    fn test_container_type_equality() {
        let container1 = create_test_container(
            "test-container",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );

        let container2 = create_test_container(
            "test-container",
            HashMap::new(),
            vec![create_test_network("bridge", "172.17.0.2")],
        );

        let warp_info1 = WarpContainerInfo {
            container: container1.clone(),
            target_network: None,
        };

        let warp_info2 = WarpContainerInfo {
            container: container2.clone(),
            target_network: None,
        };

        assert_eq!(
            ContainerType::WarpContainer(warp_info1),
            ContainerType::WarpContainer(warp_info2)
        );

        let target_info1 = TargetContainerInfo {
            container: container1,
            warp_target: "warp-1".to_string(),
        };

        let target_info2 = TargetContainerInfo {
            container: container2,
            warp_target: "warp-1".to_string(),
        };

        assert_eq!(
            ContainerType::TargetContainer(target_info1),
            ContainerType::TargetContainer(target_info2)
        );

        assert_eq!(ContainerType::Ignored, ContainerType::Ignored);
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let result = DefaultContainerClassifier::new(
            "[invalid regex".to_string(),
            "warp.target".to_string(),
            "warp.network".to_string(),
        );
        assert!(result.is_err());
    }
}
