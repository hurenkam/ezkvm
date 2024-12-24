use crate::config::default_when_missing;
use crate::config::gpu::Gpu;
use crate::config::types::QemuDevice;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct PciAddress {
    bus: u8,
    address: u8,
}
impl Default for PciAddress {
    fn default() -> Self {
        Self { bus: 0, address: 2 }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum HardwareAddress {
    #[serde(rename = "pci")]
    PCI {
        #[serde(default, deserialize_with = "default_when_missing", flatten)]
        address: PciAddress,
    },
    #[serde(rename = "pcie")]
    PCIE {
        #[serde(default, deserialize_with = "default_when_missing", flatten)]
        address: PciAddress,
    },
}

#[derive(Deserialize, Serialize, Default, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Boolean {
    #[default]
    Yes,
    No,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Virtio {
    #[serde(default, deserialize_with = "default_when_missing", flatten)]
    address: Option<HardwareAddress>,
    #[serde(default, deserialize_with = "default_when_missing")]
    vga: Boolean,
    #[serde(default = "no", deserialize_with = "default_when_missing")]
    gl: Boolean,
}
fn no() -> Boolean {
    Boolean::No
}

impl HardwareAddress {
    pub fn get_options(&self) -> String {
        match self {
            HardwareAddress::PCI { address } => {
                format!(",bus=pci.{},addr={}", address.bus, address.address)
            }
            HardwareAddress::PCIE { address } => {
                format!(",bus=pcie.{},addr={}", address.bus, address.address)
            }
        }
    }
}

impl QemuDevice for Virtio {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut options = "".to_string();
        if let Some(address) = &self.address {
            options = address.get_options();
        }
        let vga = if self.vga == Boolean::Yes {
            "vga".to_string()
        } else {
            "gpu".to_string()
        };
        let gl = if self.gl == Boolean::Yes {
            "-gl".to_string()
        } else {
            "".to_string()
        };
        vec![format!("-device virtio-{}{},id=vga{}", vga, gl, options)]
    }
}

#[typetag::deserialize(name = "virtio")]
impl Gpu for Virtio {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let actual = serde_yaml::from_str::<Virtio>(
            r#"
            "#,
        )
        .unwrap();
        let gpu = Virtio {
            address: None,
            vga: Boolean::Yes,
            gl: Boolean::No,
        };
        let expected: Vec<String> = vec!["-device virtio-vga,id=vga".to_string()];
        assert_eq!(actual, gpu);
        assert_eq!(gpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_valid_pci() {
        let actual = serde_yaml::from_str::<Virtio>(
            r#"
                pci: { bus: 1, address: 0 }
            "#,
        )
        .unwrap();
        let gpu = Virtio {
            address: Some(HardwareAddress::PCI {
                address: PciAddress { bus: 1, address: 0 },
            }),
            vga: Boolean::Yes,
            gl: Boolean::No,
        };
        println!("{}", serde_yaml::to_string(&gpu).unwrap());
        let expected: Vec<String> = vec!["-device virtio-vga,id=vga,bus=pci.1,addr=0".to_string()];
        assert_eq!(actual, gpu);
        assert_eq!(gpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_valid_pcie() {
        let actual = serde_yaml::from_str::<Virtio>(
            r#"
                pcie: { bus: 0, address: 2 }
                gl: yes
            "#,
        )
        .unwrap();
        let gpu = Virtio {
            address: Some(HardwareAddress::PCIE {
                address: PciAddress { bus: 0, address: 2 },
            }),
            vga: Boolean::Yes,
            gl: Boolean::Yes,
        };
        println!("{}", serde_yaml::to_string(&gpu).unwrap());
        let expected: Vec<String> =
            vec!["-device virtio-vga-gl,id=vga,bus=pcie.0,addr=2".to_string()];
        assert_eq!(actual, gpu);
        assert_eq!(gpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_no_address() {
        let actual = serde_yaml::from_str::<Virtio>(
            r#"
                pcie: {}
            "#,
        )
        .unwrap();
        let gpu = Virtio {
            address: Some(HardwareAddress::PCIE {
                address: PciAddress { bus: 0, address: 2 },
            }),
            vga: Boolean::Yes,
            gl: Boolean::No,
        };
        println!("{}", serde_yaml::to_string(&gpu).unwrap());
        let expected: Vec<String> = vec!["-device virtio-vga,id=vga,bus=pcie.0,addr=2".to_string()];
        assert_eq!(actual, gpu);
        assert_eq!(gpu.get_qemu_args(0), expected);
    }
}
