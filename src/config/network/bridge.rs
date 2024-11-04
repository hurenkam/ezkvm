use crate::config::network::network_payload::NetworkPayload;
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Bridge {
    #[serde(default = "Bridge::bridge_default")]
    bridge: String,
    #[serde(default = "Bridge::driver_default")]
    driver: String,
}

impl Bridge {
    required_value_getter!(bridge("bridge"): String = "vmbr0".to_string());
    required_value_getter!(driver("driver"): String = "virtio-net-pci".to_string());
}

#[typetag::deserialize(name = "bridge")]
impl NetworkPayload for Bridge {
    fn get_netdev_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "type=bridge,br={},id=hostnet{}",
            self.bridge, index
        )]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "id=net{},driver={},netdev=hostnet{},bus=pci.1,addr=0x0",
            index, self.driver, index,
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_defaults() {
        let network = Bridge {
            bridge: Bridge::bridge_default(),
            driver: Bridge::driver_default(),
        };

        let expected_netdev_options = vec!["type=bridge,br=vmbr0,id=hostnet0".to_string()];
        assert_eq!(expected_netdev_options, network.get_netdev_options(0));

        let expected_device_options =
            vec!["id=net0,driver=virtio-net-pci,netdev=hostnet0,bus=pci.1,addr=0x0".to_string()];
        assert_eq!(expected_device_options, network.get_device_options(0));
    }

    #[test]
    fn test_valid() {
        let network = Bridge {
            bridge: "vmbr2".to_string(),
            driver: "ne2000".to_string(),
        };

        let expected_netdev_options = vec!["type=bridge,br=vmbr2,id=hostnet3".to_string()];
        assert_eq!(expected_netdev_options, network.get_netdev_options(3));

        let expected_device_options =
            vec!["id=net3,driver=ne2000,netdev=hostnet3,bus=pci.1,addr=0x0".to_string()];
        assert_eq!(expected_device_options, network.get_device_options(3));
    }
}
