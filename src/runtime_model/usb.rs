use std::{fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type UsbBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct UsbAddress {
    pub port: String,
}
impl Display for UsbAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "port {}", self.port)
    }
}
#[derive(Debug, Deserialize, Serialize, Getters, new)]
pub struct UsbDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<UsbBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<UsbAddress>,
    device: UsbDeviceType,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum UsbDeviceType {
    NetworkController,
}
impl From<&UsbDeviceType> for Arc<dyn UsbDeviceApi> {
    fn from(device: &UsbDeviceType) -> Self {
        match device {
            UsbDeviceType::NetworkController => {
                todo!();
            }
        }
    }
}

pub trait UsbDeviceApi: Display {
    fn qemu_args(&self, bus: &UsbBus, address: UsbAddress) -> Vec<String>;
}
pub trait UsbControllerApi: ControllerApi + Display {
    fn register_usb_device(
        &self,
        device: Arc<dyn UsbDeviceApi>,
        preferred_address: Option<UsbAddress>,
    ) -> Result<(), String>;
}
