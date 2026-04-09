use super::network_payload::{network_pci_addr, NetworkPayload};
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

/// User-mode (slirp) network backend.
///
/// Does not require root or any host TAP setup.  The guest gets NAT'd internet
/// access through the host.  Suitable for quickly testing VMs without
/// administrative privileges.
///
/// YAML example:
/// ```yaml
/// network:
///   - type: user
///     driver: virtio-net-pci
/// ```
/// With forwarded port (QEMU hostfwd syntax):
/// ```yaml
/// network:
///   - type: user
///     hostfwd: "tcp::2222-:22"
/// ```
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct UserNet {
    #[serde(default = "UserNet::driver_default")]
    driver: String,
    /// Optional QEMU hostfwd specification, e.g. "tcp::2222-:22".
    #[serde(default)]
    hostfwd: Option<String>,
    /// Restrict guest to host-only (no external access).
    #[serde(default)]
    restrict: Option<bool>,
}

impl UserNet {
    required_value_getter!(driver("driver"): String = "virtio-net-pci".to_string());
}

#[typetag::deserialize(name = "user")]
impl NetworkPayload for UserNet {
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        let mut parts = vec!["type=user".to_string()];
        if self.restrict == Some(true) {
            parts.push("restrict=on".to_string());
        }
        if let Some(ref fwd) = self.hostfwd {
            parts.push(format!("hostfwd={}", fwd));
        }
        vec![parts.join(",")]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "{},id=net{},bus=pci.1,addr={}",
            self.driver,
            index,
            network_pci_addr(index)
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let net = UserNet {
            driver: UserNet::driver_default(),
            hostfwd: None,
            restrict: None,
        };
        assert_eq!(
            net.get_netdev_options(0),
            vec!["type=user".to_string()]
        );
        assert_eq!(
            net.get_device_options(0),
            vec!["virtio-net-pci,id=net0,bus=pci.1,addr=0x0".to_string()]
        );
    }

    #[test]
    fn test_hostfwd() {
        let net = UserNet {
            driver: UserNet::driver_default(),
            hostfwd: Some("tcp::2222-:22".to_string()),
            restrict: None,
        };
        let opts = net.get_netdev_options(0);
        assert!(opts[0].contains("hostfwd=tcp::2222-:22"));
    }

    #[test]
    fn test_restrict() {
        let net = UserNet {
            driver: UserNet::driver_default(),
            hostfwd: None,
            restrict: Some(true),
        };
        assert!(net.get_netdev_options(0)[0].contains("restrict=on"));
    }
}
