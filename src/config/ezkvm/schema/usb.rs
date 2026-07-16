use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub type UsbBusSchema = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct UsbAddressSchema {
    pub port: String,
}

impl Display for UsbAddressSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "port {}", self.port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct UsbDeviceSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<UsbBusSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<UsbAddressSchema>,
    device: UsbDeviceTypeSchema,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum UsbDeviceTypeSchema {
    NetworkController,
    Tablet,
    HostPassthrough { resource: String },
}
