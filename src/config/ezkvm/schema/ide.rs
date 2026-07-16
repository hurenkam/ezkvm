use std::fmt::Display;

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

pub type IdeBusSchema = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct IdeAddressSchema {
    pub address: u8,
}

impl Display for IdeAddressSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address {}", self.address)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct IdeDeviceSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<IdeBusSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    address: Option<IdeAddressSchema>,
    #[serde(flatten)]
    device: IdeDeviceTypeSchema,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IdeDeviceTypeSchema {
    Hdd { resource: String },
    Ssd { resource: String },
    Cdrom { resource: String },
}
