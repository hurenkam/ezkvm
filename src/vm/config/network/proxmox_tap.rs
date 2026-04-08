use super::network_payload::{network_pci_addr, NetworkPayload};
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ProxmoxTap {
    #[serde(default = "ProxmoxTap::bridge_default")]
    bridge: String,
    #[serde(default = "ProxmoxTap::vmid_default")]
    vmid: String,
    #[serde(default = "ProxmoxTap::upscript_default")]
    upscript: String,
    #[serde(default = "ProxmoxTap::downscript_default")]
    downscript: String,
    #[serde(default = "ProxmoxTap::vhost_default")]
    vhost: String,
    #[serde(default = "ProxmoxTap::driver_default")]
    driver: String,
}

impl ProxmoxTap {
    required_value_getter!(bridge("bridge"): String = "vmbr0".to_string());
    required_value_getter!(vmid("vmid"): String = "100".to_string());
    required_value_getter!(upscript("script"): String = "/usr/libexec/qemu-server/pve-bridge".to_string());
    required_value_getter!(downscript("downscript"): String = "/usr/libexec/qemu-server/pve-bridgedown".to_string());
    required_value_getter!(vhost("vhost"): String = "on".to_string());
    required_value_getter!(driver("driver"): String = "virtio-net-pci".to_string());
}

#[typetag::deserialize(name = "proxmox_tap")]
impl NetworkPayload for ProxmoxTap {
    fn get_netdev_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "type=tap,ifname=tap{}i{},script={},downscript={},vhost={}",
            self.vmid, index, self.upscript, self.downscript, self.vhost
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
        let network = ProxmoxTap {
            bridge: ProxmoxTap::bridge_default(),
            vmid: ProxmoxTap::vmid_default(),
            upscript: ProxmoxTap::upscript_default(),
            downscript: ProxmoxTap::downscript_default(),
            vhost: ProxmoxTap::vhost_default(),
            driver: ProxmoxTap::driver_default(),
        };

        let expected_netdev_options = vec![
            "type=tap,ifname=tap100i0,script=/usr/libexec/qemu-server/pve-bridge,downscript=/usr/libexec/qemu-server/pve-bridgedown,vhost=on".to_string(),
        ];
        assert_eq!(expected_netdev_options, network.get_netdev_options(0));

        let expected_device_options = vec!["virtio-net-pci,id=net0,bus=pci.1,addr=0x0".to_string()];
        assert_eq!(expected_device_options, network.get_device_options(0));
    }

    #[test]
    fn test_valid() {
        let network = ProxmoxTap {
            bridge: "vmbr2".to_string(),
            vmid: "301".to_string(),
            upscript: "/usr/libexec/qemu-server/pve-bridge".to_string(),
            downscript: "/usr/libexec/qemu-server/pve-bridgedown".to_string(),
            vhost: "off".to_string(),
            driver: "e1000".to_string(),
        };

        let expected_netdev_options = vec![
            "type=tap,ifname=tap301i3,script=/usr/libexec/qemu-server/pve-bridge,downscript=/usr/libexec/qemu-server/pve-bridgedown,vhost=off".to_string(),
        ];
        assert_eq!(expected_netdev_options, network.get_netdev_options(3));

        let expected_device_options = vec!["e1000,id=net3,bus=pci.1,addr=0x3".to_string()];
        assert_eq!(expected_device_options, network.get_device_options(3));
    }
}
