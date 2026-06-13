use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type UsbBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct UsbAddress {
    pub port: String,
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

pub trait UsbDeviceApi {
    fn qemu_args(&self, bus: &UsbBus, address: UsbAddress) -> Vec<String>;
}
pub trait UsbControllerApi: ControllerApi {
    fn register_usb_device(
        &self,
        device: Arc<dyn UsbDeviceApi>,
        preferred_address: Option<UsbAddress>,
    ) -> Result<(), String>;
}
