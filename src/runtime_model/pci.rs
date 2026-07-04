use std::any::Any;
use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

pub type PciBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PciAddress {
    pub device: u8,
    pub function: u8,
}
impl Display for PciAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct PciDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<PciBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<PciAddress>,
    device: PciDeviceType,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PciDeviceType {
    NetworkController,
    QxlGpu,
    Ac97,
}
impl From<&PciDeviceType> for Arc<dyn PciDeviceApi> {
    fn from(device: &PciDeviceType) -> Self {
        match device {
            PciDeviceType::NetworkController => {
                todo!();
            }
            PciDeviceType::QxlGpu => Arc::new(QxlGpuController::default()),
            PciDeviceType::Ac97 => Arc::new(Ac97Controller::default()),
        }
    }
}

pub trait PciDeviceApi: Display {
    fn as_any(&self) -> &dyn Any;

    fn preferred_address(&self) -> Option<PciAddress> {
        None
    }
}

// GPU Device Controllers

#[derive(Debug, Clone, Default)]
pub struct QxlGpuController {}

impl Display for QxlGpuController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QXL GPU")
    }
}

impl PciDeviceApi for QxlGpuController {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct Ac97Controller {}

impl Display for Ac97Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AC97 Audio Controller")
    }
}

impl PciDeviceApi for Ac97Controller {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait PciControllerApi: Display {
    fn register_pci_device(
        &self,
        device: Arc<dyn PciDeviceApi>,
        preferred_address: Option<PciAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<PciAddress, Arc<dyn PciDeviceApi>>;
}
