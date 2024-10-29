//use crate::yaml::QemuDevice;
use crate::config::QemuDevice;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Debug)]
pub struct Resource {
    id: String,
    tags: Vec<String>,
    pci: Vec<String>,
    parent: Option<String>,
    vf: Option<String>,
    multifunction: Option<bool>,
}

impl Resource {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl QemuDevice for Resource {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        let offset = 10;
        for (id, pci) in self.pci.iter().enumerate() {
            result.push(format!(
                "-device vfio-pci,host={},id=hostvf{},bus=ich9-pcie-port-1,addr=0x{}",
                pci,
                id,
                id + offset
            ));
        }
        result
    }
}

impl Default for Resource {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            tags: vec![],
            pci: vec![],
            parent: None,
            vf: None,
            multifunction: None,
        }
    }
}

impl<'de> Deserialize<'de> for Resource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            Tags,
            Pci,
            Parent,
            Vf,
            Multifunction,
        }

        struct PoolDeviceVisitor;

        impl<'de> Visitor<'de> for PoolDeviceVisitor {
            type Value = Resource;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct YamlConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Resource, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut pool_device = Resource::default();
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            pool_device.id = map.next_value()?;
                        }
                        Field::Tags => {
                            pool_device.tags = map.next_value()?;
                        }
                        Field::Pci => {
                            pool_device.pci = map.next_value()?;
                        }
                        Field::Multifunction => {
                            pool_device.multifunction = map.next_value()?;
                        }
                        Field::Parent => {
                            pool_device.parent = map.next_value()?;
                        }
                        Field::Vf => {
                            pool_device.vf = map.next_value()?;
                        }
                    }
                }

                Ok(pool_device)
            }
        }

        const FIELDS: &[&str] = &["id", "tags", "pci", "parent", "vf"];
        deserializer.deserialize_struct("Config", FIELDS, PoolDeviceVisitor)
    }
}
