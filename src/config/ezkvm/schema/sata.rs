use std::fmt::Display;

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

pub type SataBusSchema = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct SataAddressSchema {
    pub address: u8,
}
impl Display for SataAddressSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address {}", self.address)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct SataDeviceSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<SataBusSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    address: Option<SataAddressSchema>,
    #[serde(flatten)]
    device: SataDeviceTypeSchema,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SataDeviceTypeSchema {
    Hdd { resource: String },
    Ssd { resource: String },
    Cdrom { resource: String },
}
