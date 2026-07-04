use std::{fmt::Display, sync::Arc};

use crate::runtime_model::{StorageDeviceKind, StorageResource};

use super::PcieDeviceApi;
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
pub type ScsiBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct ScsiAddress {
    pub target: u8,
    pub lun: u8,
}
impl Display for ScsiAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "target {}, lun {}", self.target, self.lun)
    }
}

pub trait ScsiDeviceApi: Display + Send + Sync {
    fn storage_kind(&self) -> StorageDeviceKind;
    fn storage_resource(&self) -> &StorageResource;
}
pub trait ScsiControllerApi: PcieDeviceApi {
    fn register_scsi_device(
        &self,
        device: Arc<dyn ScsiDeviceApi>,
        preferred_address: Option<ScsiAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<ScsiAddress, Arc<dyn ScsiDeviceApi>>;
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct ScsiDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<ScsiBus>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    address: Option<ScsiAddress>,
    #[serde(flatten)]
    device: ScsiDeviceType,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScsiDeviceType {
    Hdd { resource: String },
    Ssd { resource: String },
    Cdrom { resource: String },
}

pub struct ScsiDeviceBuilder {}
impl ScsiDeviceBuilder {
    pub fn build(
        device_type: &ScsiDeviceType,
        storage_resources: &HashMap<String, StorageResource>,
    ) -> Result<Arc<dyn ScsiDeviceApi>, String> {
        Ok(match device_type {
            ScsiDeviceType::Hdd { resource } => Arc::new(super::Hdd::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by SCSI HDD",
                        resource
                    )
                })?,
            )),
            ScsiDeviceType::Ssd { resource } => Arc::new(super::Ssd::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by SCSI SSD",
                        resource
                    )
                })?,
            )),
            ScsiDeviceType::Cdrom { resource } => Arc::new(super::Cdrom::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by SCSI CDROM",
                        resource
                    )
                })?,
            )),
        })
    }
}
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Default)]
pub struct ScsiDisk {}

impl ScsiDeviceApi for super::Hdd {
    fn storage_kind(&self) -> StorageDeviceKind {
        StorageDeviceKind::Hdd
    }

    fn storage_resource(&self) -> &StorageResource {
        self.resource()
    }
}

impl ScsiDeviceApi for super::Ssd {
    fn storage_kind(&self) -> StorageDeviceKind {
        StorageDeviceKind::Ssd
    }

    fn storage_resource(&self) -> &StorageResource {
        self.resource()
    }
}

impl ScsiDeviceApi for super::Cdrom {
    fn storage_kind(&self) -> StorageDeviceKind {
        StorageDeviceKind::Cdrom
    }

    fn storage_resource(&self) -> &StorageResource {
        self.resource()
    }
}
