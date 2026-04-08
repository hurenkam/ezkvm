use super::super::Config;
use super::network_payload::{network_pci_addr, NetworkPayload};
use super::NetworkItem;
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

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
    required_value_getter!(parent("parent"): String = "".to_string());
    required_value_getter!(vf("vf"): String = "".to_string());
    required_value_getter!(pci("pci"): String = "".to_string());
}

#[typetag::deserialize(name = "x550vf")]
impl NetworkPayload for X550vf {
    fn pre_start(&self, _parent: &NetworkItem, _config: &Config) {
        // setup the mac address on the vm host pf interface identied by 'parent' and 'vf'
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "vfio-pci,id=net{},host={},bus=pci.1,addr={},rombar=0",
            index,
            self.pci,
            network_pci_addr(index)
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let network = X550vf {
            parent: X550vf::parent_default(),
            vf: X550vf::vf_default(),
            pci: X550vf::pci_default(),
        };

        let expected_device_options = vec![format!(
            "vfio-pci,id=net0,host={},bus=pci.1,addr=0x0,rombar=0",
            X550vf::pci_default()
        )];
        assert_eq!(expected_device_options, network.get_device_options(0));
    }

    #[test]
    fn test_valid() {
        let network = X550vf {
            parent: "enp3s0f0".to_string(),
            vf: "4".to_string(),
            pci: "0000:03:10.4".to_string(),
        };

        let expected_device_options = vec![
            "vfio-pci,id=net3,host=0000:03:10.4,bus=pci.1,addr=0x3,rombar=0".to_string(),
        ];
        assert_eq!(expected_device_options, network.get_device_options(3));
    }
}
