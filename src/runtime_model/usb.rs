use std::{any::Any, collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_model::UsbDeviceResource;

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
    Tablet,
    HostPassthrough { resource: String },
}
pub struct UsbDeviceBuilder {}
impl UsbDeviceBuilder {
    pub fn build(
        device_type: &UsbDeviceType,
        usb_resources: &HashMap<String, UsbDeviceResource>,
    ) -> Result<Arc<dyn UsbDeviceApi>, String> {
        match device_type {
            UsbDeviceType::NetworkController => Ok(Arc::new(UsbNetworkController {})),
            UsbDeviceType::Tablet => Ok(Arc::new(UsbTabletController {})),
            UsbDeviceType::HostPassthrough { resource } => {
                let usb_resource = usb_resources
                    .get(resource)
                    .ok_or_else(|| format!("USB resource '{resource}' not found"))?;

                match usb_resource {
                    UsbDeviceResource::Id {
                        vendor_id,
                        device_id,
                    } => Ok(Arc::new(UsbHostByIdController::new(*vendor_id, *device_id))),
                    UsbDeviceResource::HostBusPort { hostbus, hostport } => Ok(Arc::new(
                        UsbHostByBusPortController::new(*hostbus, hostport.clone()),
                    )),
                    UsbDeviceResource::Address { .. } => Err(
                        "USB address resources are not valid host passthrough resources"
                            .to_string(),
                    ),
                }
            }
        }
    }
}

pub struct UsbNetworkController {}

impl Display for UsbNetworkController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB Network Controller")
    }
}

pub struct UsbTabletController {}

impl Display for UsbTabletController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB Tablet")
    }
}

#[derive(Debug, Clone)]
pub struct UsbHostByIdController {
    vendor_id: u16,
    device_id: u16,
}

impl UsbHostByIdController {
    pub fn new(vendor_id: u16, device_id: u16) -> Self {
        Self {
            vendor_id,
            device_id,
        }
    }

    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    pub fn device_id(&self) -> u16 {
        self.device_id
    }
}

impl Display for UsbHostByIdController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "USB Host Passthrough {:04x}:{:04x}",
            self.vendor_id, self.device_id
        )
    }
}

#[derive(Debug, Clone)]
pub struct UsbHostByBusPortController {
    hostbus: u16,
    hostport: String,
}

impl UsbHostByBusPortController {
    pub fn new(hostbus: u16, hostport: String) -> Self {
        Self { hostbus, hostport }
    }

    pub fn hostbus(&self) -> u16 {
        self.hostbus
    }

    pub fn hostport(&self) -> &str {
        &self.hostport
    }
}

impl Display for UsbHostByBusPortController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB Host Passthrough {}-{}", self.hostbus, self.hostport)
    }
}

pub trait UsbDeviceApi: Display {
    fn as_any(&self) -> &dyn Any;
}

impl UsbDeviceApi for UsbNetworkController {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl UsbDeviceApi for UsbTabletController {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl UsbDeviceApi for UsbHostByIdController {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl UsbDeviceApi for UsbHostByBusPortController {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait UsbControllerApi: Display {
    fn register_usb_device(
        &self,
        device: Arc<dyn UsbDeviceApi>,
        preferred_address: Option<UsbAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<UsbAddress, Arc<dyn UsbDeviceApi>>;
}
