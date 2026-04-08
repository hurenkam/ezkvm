use super::super::Config;
use super::NetworkItem;
use std::fmt::Debug;

pub fn network_pci_addr(index: usize) -> String {
    // Keep NIC slots stable by index and skip 0x1b, used by qemu-xhci on pci.1.
    let mut addr = index;
    if addr >= 0x1b {
        addr += 1;
    }

    format!("0x{:x}", addr)
}

#[typetag::deserialize(tag = "type")]
pub trait NetworkPayload: Debug {
    fn pre_start(&self, _parent: &NetworkItem, _config: &Config) {}
    fn post_start(&self, _parent: &NetworkItem, _config: &Config) {}
    fn pre_stop(&self, _parent: &NetworkItem, _config: &Config) {}
    fn post_stop(&self, _parent: &NetworkItem, _config: &Config) {}
    fn pre_hibernate(&self, _parent: &NetworkItem, _config: &Config) {}
    fn post_hibernate(&self, _parent: &NetworkItem, _config: &Config) {}
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
    fn get_device_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::network_pci_addr;

    #[test]
    fn test_network_pci_addr() {
        assert_eq!(network_pci_addr(0), "0x0");
        assert_eq!(network_pci_addr(3), "0x3");
        assert_eq!(network_pci_addr(0x1b), "0x1c");
    }
}
