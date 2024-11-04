use crate::config::network::network_payload::NetworkPayload;
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Tap {
    #[serde(default = "Tap::ifname_default")]
    ifname: String,
    #[serde(default = "Tap::upscript_default")]
    upscript: String,
    #[serde(default = "Tap::downscript_default")]
    downscript: String,
    #[serde(default = "Tap::vhost_default")]
    vhost: String,
    #[serde(default = "Tap::driver_default")]
    driver: String,
}

impl Tap {
    required_value_getter!(ifname("ifname"): String = "tap0".to_string());
    required_value_getter!(upscript("script"): String = "/var/lib/qemu/bridge-up".to_string());
    required_value_getter!(downscript("downscript"): String = "/var/lib/qemu/bridge-down".to_string());
    required_value_getter!(vhost("vhost"): String = "on".to_string());
    required_value_getter!(driver("driver"): String = "virtio-net-pci".to_string());
}

#[typetag::deserialize(name = "tap")]
impl NetworkPayload for Tap {
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        vec![format!(
            "type=tap,script={},downscript={},vhost={}",
            self.upscript, self.downscript, self.vhost
        )]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "{},id=net{},bus=pci.1,addr=0x0",
            self.driver, index
        )]
    }
}

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
