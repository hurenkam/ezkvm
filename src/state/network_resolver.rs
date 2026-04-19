use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkResolutionMode {
    Unchanged,
    BridgeHelper,
    UserFallback,
}

#[derive(Debug, Clone)]
pub struct ResolvedNetworkOutcome {
    pub network: crate::config::NetworkConfig,
    pub mode: NetworkResolutionMode,
    pub warning: Option<String>,
}

pub trait NetworkCapabilityResolver {
    fn resolve_network(
        &self,
        vm_name: &str,
        network: &crate::config::NetworkConfig,
    ) -> ResolvedNetworkOutcome;
}

pub struct CentralNetworkCapabilityResolver<'a> {
    central_config: &'a crate::config::CentralConfig,
}

impl<'a> CentralNetworkCapabilityResolver<'a> {
    pub fn new(central_config: &'a crate::config::CentralConfig) -> Self {
        Self { central_config }
    }

    fn non_empty(value: Option<&str>) -> Option<&str> {
        value.and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    fn find_in_path(program: &str) -> Option<String> {
        let path = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(program);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
        None
    }

    fn is_file(path: &str) -> bool {
        Path::new(path).is_file()
    }

    fn resolve_bridge_helper(&self, explicit: Option<&str>) -> Option<String> {
        if let Some(path) = Self::non_empty(explicit) {
            return if Self::is_file(path) {
                Some(path.to_string())
            } else {
                None
            };
        }

        if let Some(path) = Self::non_empty(self.central_config.bridge_helper()) {
            return if Self::is_file(path) {
                Some(path.to_string())
            } else {
                None
            };
        }

        if let Some(found) = Self::find_in_path("qemu-bridge-helper") {
            return Some(found);
        }

        for candidate in [
            "/usr/lib/qemu/qemu-bridge-helper",
            "/usr/libexec/qemu-bridge-helper",
            "/usr/lib64/qemu-bridge-helper",
        ] {
            if Self::is_file(candidate) {
                return Some(candidate.to_string());
            }
        }

        None
    }

    fn prefers_user_mode(&self) -> bool {
        matches!(
            self.central_config
                .network_backend_preference()
                .map(str::trim),
            Some("user") | Some("user-mode")
        )
    }

    fn user_backend(vm_name: &str) -> crate::config::NetworkBackendConfig {
        let mut extra = BTreeMap::new();
        extra.insert("hostname".to_string(), vm_name.to_string());

        crate::config::NetworkBackendConfig {
            backend_type: "user".to_string(),
            extra,
            ..Default::default()
        }
    }
}

impl NetworkCapabilityResolver for CentralNetworkCapabilityResolver<'_> {
    fn resolve_network(
        &self,
        vm_name: &str,
        network: &crate::config::NetworkConfig,
    ) -> ResolvedNetworkOutcome {
        let Some(backend) = network.backend.as_ref() else {
            return ResolvedNetworkOutcome {
                network: network.clone(),
                mode: NetworkResolutionMode::Unchanged,
                warning: None,
            };
        };

        if backend.backend_type != "bridge" {
            return ResolvedNetworkOutcome {
                network: network.clone(),
                mode: NetworkResolutionMode::Unchanged,
                warning: None,
            };
        }

        if self.prefers_user_mode() {
            let mut resolved = network.clone();
            resolved.backend = Some(Self::user_backend(vm_name));
            return ResolvedNetworkOutcome {
                network: resolved,
                mode: NetworkResolutionMode::UserFallback,
                warning: Some(format!(
                    "network '{}' downgraded to user-mode because host_capabilities.network.preferred_backend is set to user/user-mode",
                    network.id
                )),
            };
        }

        if let Some(helper) = self.resolve_bridge_helper(backend.helper.as_deref()) {
            let mut resolved = network.clone();
            let mut resolved_backend = backend.clone();
            resolved_backend.helper = Some(helper);

            if resolved_backend.bridge.is_none()
                && let Some(default_bridge) = self.central_config.bridge_name()
            {
                resolved_backend.bridge = Some(default_bridge.to_string());
            }

            resolved.backend = Some(resolved_backend);
            return ResolvedNetworkOutcome {
                network: resolved,
                mode: NetworkResolutionMode::BridgeHelper,
                warning: None,
            };
        }

        let mut resolved = network.clone();
        resolved.backend = Some(Self::user_backend(vm_name));

        ResolvedNetworkOutcome {
            network: resolved,
            mode: NetworkResolutionMode::UserFallback,
            warning: Some(format!(
                "network '{}' downgraded to user-mode because no bridge helper was found; install qemu-bridge-helper or set host_capabilities.network.bridge_helper",
                network.id
            )),
        }
    }
}

pub fn resolve_networks_for_vm(
    vm_name: &str,
    devices: &mut crate::config::DeviceConfig,
    central_config: &crate::config::CentralConfig,
) -> Vec<String> {
    let resolver = CentralNetworkCapabilityResolver::new(central_config);
    let mut warnings = Vec::new();

    for network in &mut devices.networks {
        let outcome = resolver.resolve_network(vm_name, network);
        *network = outcome.network;
        if let Some(warning) = outcome.warning {
            warnings.push(warning);
        }
    }

    warnings
}

pub fn resolve_network_outcome(
    vm_name: &str,
    network: &crate::config::NetworkConfig,
    central_config: &crate::config::CentralConfig,
) -> ResolvedNetworkOutcome {
    CentralNetworkCapabilityResolver::new(central_config).resolve_network(vm_name, network)
}

#[cfg(test)]
mod tests {
    use super::{NetworkResolutionMode, resolve_network_outcome};

    fn bridge_network(id: &str) -> crate::config::NetworkConfig {
        crate::config::NetworkConfig {
            id: id.to_string(),
            model: "virtio-net-pci".to_string(),
            backend: Some(crate::config::NetworkBackendConfig {
                backend_type: "bridge".to_string(),
                bridge: Some("vmbr0".to_string()),
                ..Default::default()
            }),
            mac: None,
            rx_queue_size: None,
            tx_queue_size: None,
            boot_index: None,
            bus: None,
            addr: None,
        }
    }

    #[test]
    fn falls_back_to_user_mode_when_helper_missing() {
        let network = bridge_network("net0");
        let outcome = resolve_network_outcome(
            "vm-demo",
            &network,
            &crate::config::CentralConfig::default(),
        );

        assert_eq!(outcome.mode, NetworkResolutionMode::UserFallback);
        assert!(outcome.warning.is_some());
        assert_eq!(
            outcome
                .network
                .backend
                .as_ref()
                .expect("backend should exist")
                .backend_type,
            "user"
        );
    }

    #[test]
    fn uses_user_mode_when_preference_is_user() {
        let network = bridge_network("net0");
        let mut central = crate::config::CentralConfig::default();
        central.host_capabilities.network.preferred_backend = Some("user-mode".to_string());

        let outcome = resolve_network_outcome("vm-demo", &network, &central);
        assert_eq!(outcome.mode, NetworkResolutionMode::UserFallback);
        assert!(outcome.warning.is_some());
    }

    #[test]
    fn keeps_non_bridge_backends_unchanged() {
        let network = crate::config::NetworkConfig {
            id: "net1".to_string(),
            model: "virtio-net-pci".to_string(),
            backend: Some(crate::config::NetworkBackendConfig {
                backend_type: "user".to_string(),
                ..Default::default()
            }),
            mac: None,
            rx_queue_size: None,
            tx_queue_size: None,
            boot_index: None,
            bus: None,
            addr: None,
        };

        let outcome = resolve_network_outcome(
            "vm-demo",
            &network,
            &crate::config::CentralConfig::default(),
        );

        assert_eq!(outcome.mode, NetworkResolutionMode::Unchanged);
        assert!(outcome.warning.is_none());
        assert_eq!(
            outcome
                .network
                .backend
                .as_ref()
                .expect("backend should exist")
                .backend_type,
            "user"
        );
    }
}
