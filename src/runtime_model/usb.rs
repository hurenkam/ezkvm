use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_model::UsbDeviceResource;

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
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct UsbDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<UsbBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<UsbAddress>,
    device: UsbDeviceType,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum UsbDeviceType {
    NetworkController,
}
pub struct UsbDeviceBuilder {}
impl UsbDeviceBuilder {
    pub fn build(
        device_type: &UsbDeviceType,
        _usb_resources: &HashMap<String, UsbDeviceResource>,
    ) -> Arc<dyn UsbDeviceApi> {
        match device_type {
            UsbDeviceType::NetworkController => Arc::new(UsbNetworkController {}),
        }
    }
}

pub struct UsbNetworkController {}

impl Display for UsbNetworkController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB Network Controller")
    }
}

impl UsbDeviceApi for UsbNetworkController {
    fn qemu_args(&self, _bus: &UsbBus, _address: UsbAddress) -> Vec<String> {
        todo!()
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
    fn devices(&self) -> HashMap<UsbAddress, Arc<dyn UsbDeviceApi>>;
}
