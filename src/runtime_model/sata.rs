use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::{
    runtime_config::StorageResource,
    runtime_model::{Cdrom, Hdd, Ssd},
};

use super::ControllerApi;

pub type SataBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct SataAddress {
    pub address: u8,
}
impl Display for SataAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address {}", self.address)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct SataDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<SataBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    address: Option<SataAddress>,
    #[serde(flatten)]
    device: SataDeviceType,
}
pub struct SataDeviceBuilder {}
impl SataDeviceBuilder {
    pub fn build(
        device_type: &SataDeviceType,
        storage_resources: &HashMap<String, StorageResource>,
    ) -> Result<Arc<dyn SataDeviceApi + 'static>, String> {
        Ok(match device_type {
            SataDeviceType::Hdd { resource } => Arc::new(Hdd::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by SATA HDD",
                        resource
                    )
                })?,
            )),
            SataDeviceType::Ssd { resource } => Arc::new(super::Ssd::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by SATA SSD",
                        resource
                    )
                })?,
            )),
            SataDeviceType::Cdrom { resource } => Arc::new(super::Cdrom::new(
                storage_resources.get(resource).cloned().ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by SATA CDROM",
                        resource
                    )
                })?,
            )),
        })
    }
}
pub trait SataDeviceApi: Display {
    fn qemu_args(&self, assigned_bus: &SataBus, assigned_address: SataAddress) -> Vec<String>;
}

pub trait SataControllerApi: ControllerApi + Display {
    fn register_sata_device(
        &self,
        device: Arc<dyn SataDeviceApi>,
        preferred_address: Option<SataAddress>,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SataDeviceType {
    Hdd { resource: String },
    Ssd { resource: String },
    Cdrom { resource: String },
}

impl SataDeviceApi for Hdd {
    fn qemu_args(&self, _assigned_bus: &SataBus, _assigned_address: SataAddress) -> Vec<String> {
        todo!()
    }
}

impl SataDeviceApi for Ssd {
    fn qemu_args(&self, _assigned_bus: &SataBus, _assigned_address: SataAddress) -> Vec<String> {
        todo!()
    }
}

impl SataDeviceApi for Cdrom {
    fn qemu_args(&self, _assigned_bus: &SataBus, _assigned_address: SataAddress) -> Vec<String> {
        todo!()
    }
}
