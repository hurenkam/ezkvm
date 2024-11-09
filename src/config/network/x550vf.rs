use crate::config::network::network_payload::NetworkPayload;
use crate::config::Config;
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};
use crate::config::network::NetworkItem;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct X550vf {
    #[serde(default = "X550vf::parent_default")]
    parent: String,
    #[serde(default = "X550vf::vf_default")]
    vf: String,
    #[serde(default = "X550vf::pci_default")]
    pci: String,
}

impl X550vf {
    required_value_getter!(parent("parent"): String = "tap0".to_string());
    required_value_getter!(vf("vf"): String = "/var/lib/qemu/bridge-up".to_string());
    required_value_getter!(pci("pci"): String = "/var/lib/qemu/bridge-down".to_string());
}

#[typetag::deserialize(name = "x550vf")]
impl NetworkPayload for X550vf {
    fn pre_start(&self, _parent: &NetworkItem, _config: &Config) {
        // setup the mac address on the vm host pf interface identied by 'parent' and 'vf'
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "vfio-pci,id=net{},host={},bus=pci.1,addr=0x0,rombar=0",
            index, self.pci
        )]
    }
}
/*
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_defaults() {
        let network = Tap {
            ifname: Tap::ifname_default(),
            upscript: Tap::upscript_default(),
            downscript: Tap::downscript_default(),
            vhost: Tap::vhost_default(),
            driver: Tap::driver_default(),
        };

        let expected_netdev_options = vec![
            "type=tap,script=/var/lib/qemu/bridge-up,downscript=/var/lib/qemu/bridge-down,vhost=on"
                .to_string(),
        ];
        assert_eq!(expected_netdev_options, network.get_netdev_options(0));

        let expected_device_options = vec!["virtio-net-pci,id=net0,bus=pci.1,addr=0x0".to_string()];
        assert_eq!(expected_device_options, network.get_device_options(0));
    }

    #[test]
    fn test_valid() {
        let network = Tap {
            ifname: "tap_401i0".to_string(),
            upscript: "~/.ezkvm/upscript".to_string(),
            downscript: "~/.ezkvm/downscript".to_string(),
            vhost: "off".to_string(),
            driver: "ne2000".to_string(),
        };

        let expected_netdev_options = vec![
            "type=tap,script=~/.ezkvm/upscript,downscript=~/.ezkvm/downscript,vhost=off"
                .to_string(),
        ];
        assert_eq!(expected_netdev_options, network.get_netdev_options(3));

        let expected_device_options = vec!["ne2000,id=net3,bus=pci.1,addr=0x0".to_string()];
        assert_eq!(expected_device_options, network.get_device_options(3));
    }
}
*/
