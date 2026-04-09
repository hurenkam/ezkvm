use super::network_payload::{network_pci_addr, NetworkPayload};
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

/// Macvtap/macvlan network backend.
///
/// Attaches the guest NIC directly to a host network interface using the
/// macvtap kernel module.  The host opens `/dev/tapN` and passes the file
/// descriptor to QEMU.  This provides native-interface performance without
/// a userspace bridge.
///
/// **Prerequisites**: macvtap kernel module loaded, a macvtap interface
/// created on the host (e.g. `ip link add link eth0 name macvtap0 type macvtap`),
/// and the tap device path must be accessible.
///
/// YAML example:
/// ```yaml
/// network:
///   - type: macvtap
///     tap_dev: /dev/tap5
///     driver: virtio-net-pci
/// ```
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct MacvtapNet {
    /// Path to the macvtap device, e.g. `/dev/tap5`.
    #[serde(default = "MacvtapNet::tap_dev_default")]
    tap_dev: String,
    #[serde(default = "MacvtapNet::driver_default")]
    driver: String,
    #[serde(default = "MacvtapNet::vhost_default")]
    vhost: bool,
}

impl MacvtapNet {
    required_value_getter!(tap_dev("tap_dev"): String = "/dev/tap0".to_string());
    required_value_getter!(driver("driver"): String = "virtio-net-pci".to_string());

    fn vhost_default() -> bool {
        true
    }
}

#[typetag::deserialize(name = "macvtap")]
impl NetworkPayload for MacvtapNet {
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        let vhost_str = if self.vhost { "on" } else { "off" };
        vec![format!(
            "type=tap,fd=3,vhost={}",
            vhost_str
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
        let net = MacvtapNet {
            tap_dev: MacvtapNet::tap_dev_default(),
            driver: MacvtapNet::driver_default(),
            vhost: true,
        };
        assert_eq!(
            net.get_netdev_options(0),
            vec!["type=tap,fd=3,vhost=on".to_string()]
        );
        assert_eq!(
            net.get_device_options(0),
            vec!["virtio-net-pci,id=net0,bus=pci.1,addr=0x0".to_string()]
        );
    }

    #[test]
    fn test_vhost_off() {
        let net = MacvtapNet {
            tap_dev: "/dev/tap5".to_string(),
            driver: "e1000".to_string(),
            vhost: false,
        };
        assert!(net.get_netdev_options(0)[0].contains("vhost=off"));
    }
}
