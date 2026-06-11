use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type UsbBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct UsbAddress {
    pub port: String,
}

pub trait UsbDeviceApi {
    fn qemu_args(&self, bus: &UsbBus, address: UsbAddress) -> Vec<String>;
}
pub trait UsbControllerApi: ControllerApi {
    fn register_usb_device(
        &self,
        device: Arc<dyn UsbDeviceApi>,
        preferred_address: UsbAddress,
    ) -> Result<(), String>;
}
