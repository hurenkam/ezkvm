use crate::config::network::Network;
use crate::config::types::QemuDevice;
use rand::Rng;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Bridge {
    #[serde(default = "default_bridge")]
    bridge: String,
    #[serde(default = "default_driver")]
    driver: String,
    #[serde(default = "default_mac")]
    mac: String,
}

#[typetag::deserialize(name = "bridge")]
impl Network for Bridge {}

impl Default for Bridge {
    fn default() -> Self {
        Self {
            bridge: default_bridge(),
            driver: default_driver(),
            mac: default_mac(),
        }
    }
}

fn default_bridge() -> String {
    "vmbr0".to_string()
}

fn default_driver() -> String {
    "virtio-net-pci".to_string()
}

fn default_mac() -> String {
    let mut rng = rand::thread_rng();
    let mut result: Vec<String> = vec![];
    for _ in 0..=5 {
        result.push(format!("{:02X}", rng.gen_range(0..=255)));
    }
    result.join(":")
}

impl QemuDevice for Bridge {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-netdev id=hostnet{},type=bridge,br={}", index, self.bridge),
            format!(
                "-device id=net{},driver={},netdev=hostnet{},mac={},bus=pci.1,addr=0x0",
                index, self.driver, index, self.mac
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let network = Bridge::default();
        let expected: Vec<String> = vec![
            "-netdev id=hostnet0,type=bridge,br=vmbr0".to_string(),
            format!(
                "-device id=net0,driver=virtio-net-pci,netdev=hostnet0,mac={},bus=pci.1,addr=0x0",
                network.mac
            ),
        ];
        assert_eq!(network.get_qemu_args(0), expected);
    }

    #[test]
    fn test_valid() {
        let network = Bridge {
            bridge: "vmbr0".to_string(),
            driver: "virtio-net-pci".to_string(),
            mac: "BC:24:11:FF:76:89".to_string(),
        };
        let expected: Vec<String> = vec![
            "-netdev id=hostnet0,type=bridge,br=vmbr0".to_string(),
            "-device id=net0,driver=virtio-net-pci,netdev=hostnet0,mac=BC:24:11:FF:76:89,bus=pci.1,addr=0x0".to_string()
        ];
        assert_eq!(network.get_qemu_args(0), expected);
    }
}
