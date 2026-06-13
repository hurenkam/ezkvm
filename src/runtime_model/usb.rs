use std::{fmt::Display, sync::Arc};

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type UsbBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct UsbAddress {
    pub port: String,
}
impl Display for UsbAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "port {}", self.port)
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub enum UsbDevice {
    NetworkController,
}
impl From<UsbDevice> for Arc<dyn UsbDeviceApi> {
    fn from(device: UsbDevice) -> Self {
        match device {
            UsbDevice::NetworkController => {
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
