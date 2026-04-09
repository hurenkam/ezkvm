use super::network_payload::{network_pci_addr, NetworkPayload};
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

/// Vhost-user network backend.
///
/// Connects QEMU to a separate vhost-user process (e.g. DPDK virtio-user,
/// OVS-DPDK, Snabb, or a vhost-user-net daemon) via a Unix socket.
/// Delivers near-native throughput by bypassing the host kernel entirely.
///
/// **Prerequisites**: a running vhost-user backend process listening on the
/// configured socket path.
///
/// YAML example:
/// ```yaml
/// network:
///   - type: vhost_user
///     socket_path: /var/run/vhost-net0.sock
///     driver: virtio-net-pci
/// ```
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct VhostUserNet {
    /// Path to the vhost-user Unix domain socket.
    #[serde(default = "VhostUserNet::socket_path_default")]
    socket_path: String,
    #[serde(default = "VhostUserNet::driver_default")]
    driver: String,
}

impl VhostUserNet {
    required_value_getter!(socket_path("socket_path"): String = "/var/run/vhost-net0.sock".to_string());
    required_value_getter!(driver("driver"): String = "virtio-net-pci".to_string());
}

#[typetag::deserialize(name = "vhost_user")]
impl NetworkPayload for VhostUserNet {
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        vec![format!(
            "type=vhost-user,chardev=chr-vu,vhostforce=on"
        )]
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
        let net = VhostUserNet {
            socket_path: VhostUserNet::socket_path_default(),
            driver: VhostUserNet::driver_default(),
        };
        let netdev_opts = net.get_netdev_options(0);
        assert!(netdev_opts[0].contains("type=vhost-user"));
        assert_eq!(
            net.get_device_options(0),
            vec!["virtio-net-pci,id=net0,bus=pci.1,addr=0x0".to_string()]
        );
    }

    #[test]
    fn test_from_yaml() {
        let yaml = r#"
            socket_path: /var/run/vhost-net1.sock
            driver: virtio-net-pci
        "#;
        let net: VhostUserNet = serde_yaml::from_str(yaml).unwrap();
        let opts = net.get_netdev_options(0);
        assert!(opts[0].contains("type=vhost-user"));
    }
}
