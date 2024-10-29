//use crate::yaml::QemuDevice;
use crate::config::QemuDevice;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Debug)]
pub struct Gpu {
    driver: String,
    memory: u64,
    pci_address: String,
}

impl QemuDevice for Gpu {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        match self.driver.as_str() {
            "qxl-vga" => {
                vec![
                    format!("-device {},id=vga,vgamem_mb={},ram_size_mb={},vram_size_mb={},bus=pcie.0,addr={}",
                            self.driver, self.memory, self.memory*4, self.memory*2, self.pci_address),
                ]
            }
            "virtio-gpu-pci" => {
                vec![format!(
                    "-device {},id=vga,bus=pcie.0,addr={}",
                    self.driver, self.pci_address
                )]
            }
            "virtio-vga-gl" => {
                vec![format!(
                    "-device {},id=vga,bus=pcie.0,addr={}",
                    self.driver, self.pci_address
                )]
            }
            _ => {
                vec![]
            }
        }
    }
}

impl Default for Gpu {
    fn default() -> Self {
        Self {
            driver: "qxl-vga".to_string(),
            memory: 64,
            pci_address: "0x2".to_string(),
        }
    }
}

impl<'de> Deserialize<'de> for Gpu {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Driver,
            Memory,
            PciAddress,
        }

        struct GpuVisitor;

        impl<'de> Visitor<'de> for GpuVisitor {
            type Value = Gpu;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct YamlConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Gpu, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut gpu = Gpu::default();

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Driver => {
                            gpu.driver = map.next_value()?;
                        }
                        Field::Memory => {
                            gpu.memory = map.next_value()?;
                        }
                        Field::PciAddress => {
                            gpu.pci_address = map.next_value()?;
                        }
                    }
                }

                Ok(gpu)
            }
        }

        const FIELDS: &[&str] = &["name", "uuid"];
        deserializer.deserialize_struct("YamlConfig", FIELDS, GpuVisitor)
    }
}
