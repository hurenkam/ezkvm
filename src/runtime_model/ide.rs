use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_config::StorageResource;
use crate::runtime_model::ControllerApi;

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
    fn qemu_args(&self, assigned_bus: &IdeBus, assigned_address: IdeAddress) -> Vec<String> {
        ide_drive_args(
            self.resource(),
            *assigned_bus,
            assigned_address.address,
            "ide-hd",
            false,
        )
    }
}

impl IdeDeviceApi for super::Ssd {
    fn qemu_args(&self, assigned_bus: &IdeBus, assigned_address: IdeAddress) -> Vec<String> {
        ide_drive_args(
            self.resource(),
            *assigned_bus,
            assigned_address.address,
            "ide-hd",
            false,
        )
    }
}

impl IdeDeviceApi for super::Cdrom {
    fn qemu_args(&self, assigned_bus: &IdeBus, assigned_address: IdeAddress) -> Vec<String> {
        ide_drive_args(
            self.resource(),
            *assigned_bus,
            assigned_address.address,
            "ide-cd",
            true,
        )
    }
}

pub trait IdeDeviceApi: Display {
    fn qemu_args(&self, assigned_bus: &IdeBus, assigned_address: IdeAddress) -> Vec<String>;
}

pub trait IdeControllerApi: ControllerApi + Display {
    fn register_ide_device(
        &self,
        device: Arc<dyn IdeDeviceApi>,
        preferred_address: Option<IdeAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<IdeAddress, Arc<dyn IdeDeviceApi>>;
}

fn ide_drive_args(
    resource: &StorageResource,
    bus: IdeBus,
    unit: u8,
    device_type: &str,
    media_cdrom: bool,
) -> Vec<String> {
    let drive_id = format!("drive-ide{unit}");
    let device_id = format!("ide{unit}");
    let mut drive_options = vec!["if=none".to_string(), format!("id={drive_id}")];

    match resource {
        StorageResource::File { file } => drive_options.push(format!("file={file}")),
        StorageResource::BlockDevice { block_device } => {
            drive_options.push(format!("file={block_device}"))
        }
    }

    drive_options.push("format=raw".to_string());
    if media_cdrom {
        drive_options.push("media=cdrom".to_string());
        drive_options.push("readonly=on".to_string());
    }

    vec![
        "-drive".to_string(),
        drive_options.join(","),
        "-device".to_string(),
        format!("{device_type},bus=ide.{bus},unit={unit},drive={drive_id},id={device_id}"),
    ]
}
