use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_model::{StorageDeviceKind, StorageResource};

pub type IdeBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct IdeAddress {
    pub address: u8,
}
impl Display for IdeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address {}", self.address)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct IdeDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<IdeBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    address: Option<IdeAddress>,
    #[serde(flatten)]
    device: IdeDeviceType,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IdeDeviceType {
    Hdd { resource: String },
    Ssd { resource: String },
    Cdrom { resource: String },
}
pub struct IdeDeviceBuilder {}
impl IdeDeviceBuilder {
    pub fn build(
        device_type: &IdeDeviceType,
        storage_resources: &HashMap<String, StorageResource>,
    ) -> Result<Arc<dyn IdeDeviceApi>, String> {
        Ok(match device_type {
            IdeDeviceType::Hdd { resource } => Arc::new(super::Hdd::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by IDE HDD",
                        resource
                    )
                })?,
            )),
            IdeDeviceType::Ssd { resource } => Arc::new(super::Ssd::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by IDE SSD",
                        resource
                    )
                })?,
            )),
            IdeDeviceType::Cdrom { resource } => Arc::new(super::Cdrom::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by IDE CDROM",
                        resource
                    )
                })?,
            )),
        })
    }
}
impl IdeDeviceApi for super::Hdd {
    fn storage_kind(&self) -> StorageDeviceKind {
        StorageDeviceKind::Hdd
    }

    fn storage_resource(&self) -> &StorageResource {
        self.resource()
    }
}

impl IdeDeviceApi for super::Ssd {
    fn storage_kind(&self) -> StorageDeviceKind {
        StorageDeviceKind::Ssd
    }

    fn storage_resource(&self) -> &StorageResource {
        self.resource()
    }
}

impl IdeDeviceApi for super::Cdrom {
    fn storage_kind(&self) -> StorageDeviceKind {
        StorageDeviceKind::Cdrom
    }

    fn storage_resource(&self) -> &StorageResource {
        self.resource()
    }
}

pub trait IdeDeviceApi: Display {
    fn storage_kind(&self) -> StorageDeviceKind;
    fn storage_resource(&self) -> &StorageResource;
}

pub trait IdeControllerApi: Display {
    fn register_ide_device(
        &self,
        device: Arc<dyn IdeDeviceApi>,
        preferred_address: Option<IdeAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<IdeAddress, Arc<dyn IdeDeviceApi>>;
}
