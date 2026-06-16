use std::{fmt::Display, sync::Arc};

use super::{ControllerApi, PcieDeviceApi};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_config::StorageResource;
use std::collections::HashMap;
pub type ScsiBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct ScsiAddress {
    pub target: u8,
    pub lun: u8,
}
impl Display for ScsiAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "target {}, lun {}", self.target, self.lun)
    }
}

pub trait ScsiDeviceApi: Display {
    fn qemu_args(&self, assigned_bus: &ScsiBus, assigned_address: ScsiAddress) -> Vec<String>;
}
pub trait ScsiControllerApi: ControllerApi + PcieDeviceApi {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<ScsiAddress>,
    device: ScsiDeviceType,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Default)]
pub struct ScsiDisk {}

impl ScsiDeviceApi for super::Hdd {
    fn qemu_args(&self, _assigned_bus: &ScsiBus, _assigned_address: ScsiAddress) -> Vec<String> {
        todo!()
    }
}

impl ScsiDeviceApi for super::Ssd {
    fn qemu_args(&self, _assigned_bus: &ScsiBus, _assigned_address: ScsiAddress) -> Vec<String> {
        todo!()
    }
}

impl ScsiDeviceApi for super::Cdrom {
    fn qemu_args(&self, _assigned_bus: &ScsiBus, _assigned_address: ScsiAddress) -> Vec<String> {
        todo!()
    }
}
