use std::{fmt::Display, sync::Arc};

use super::{ControllerApi, PcieDeviceApi};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_config::StorageResource;
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
#[derive(Default)]
pub struct ScsiDisk {}

impl ScsiDeviceApi for super::Hdd {
    fn qemu_args(&self, assigned_bus: &ScsiBus, assigned_address: ScsiAddress) -> Vec<String> {
        scsi_drive_args(
            self.resource(),
            *assigned_bus,
            assigned_address,
            "scsi-hd",
            false,
        )
    }
}

impl ScsiDeviceApi for super::Ssd {
    fn qemu_args(&self, assigned_bus: &ScsiBus, assigned_address: ScsiAddress) -> Vec<String> {
        scsi_drive_args(
            self.resource(),
            *assigned_bus,
            assigned_address,
            "scsi-hd",
            false,
        )
    }
}

impl ScsiDeviceApi for super::Cdrom {
    fn qemu_args(&self, assigned_bus: &ScsiBus, assigned_address: ScsiAddress) -> Vec<String> {
        scsi_drive_args(
            self.resource(),
            *assigned_bus,
            assigned_address,
            "scsi-cd",
            true,
        )
    }
}

fn scsi_drive_args(
    resource: &StorageResource,
    bus: ScsiBus,
    address: ScsiAddress,
    device_type: &str,
    media_cdrom: bool,
) -> Vec<String> {
    let drive_id = format!("drive-scsi{}", address.lun);
    let device_id = format!("scsi{}", address.lun);
    let mut drive_options = vec![format!("id={drive_id}")];

    match resource {
        StorageResource::File { file } => drive_options.push(format!("file={file}")),
        StorageResource::BlockDevice { block_device } => {
            drive_options.push(format!("file={block_device}"))
        }
    }

    drive_options.push("if=none".to_string());
    drive_options.push("format=raw".to_string());
    drive_options.push("discard=unmap".to_string());
    drive_options.push("detect-zeroes=unmap".to_string());
    if media_cdrom {
        drive_options.push("media=cdrom".to_string());
        drive_options.push("readonly=on".to_string());
    }

    vec![
        "-drive".to_string(),
        drive_options.join(","),
        "-device".to_string(),
        format!(
            "{device_type},bus=scsihw{bus}.0,channel=0,scsi-id={},lun={},drive={drive_id},id={device_id}",
            address.target, address.lun
        ),
    ]
}
