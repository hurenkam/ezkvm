use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub type ScsiBusSchema = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct ScsiAddressSchema {
    pub target: u8,
    pub lun: u8,
}

impl Display for ScsiAddressSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "target {}, lun {}", self.target, self.lun)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct ScsiDeviceSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<ScsiBusSchema>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    address: Option<ScsiAddressSchema>,
    #[serde(flatten)]
    device: ScsiDeviceTypeSchema,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScsiDeviceTypeSchema {
    Hdd { resource: String },
    Ssd { resource: String },
    Cdrom { resource: String },
}
