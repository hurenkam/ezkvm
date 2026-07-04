use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::{collections::HashMap, fmt::Display, sync::Arc};

pub type PcieBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct PcieAddress {
    device: u8,
    function: u8,
}
impl PcieAddress {
    pub fn device(&self) -> u8 {
        self.device
    }

    pub fn function(&self) -> u8 {
        self.function
    }
}
impl Display for PcieAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct PcieDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<PcieBus>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    address: Option<PcieAddress>,
    #[serde(flatten)]
    device: PcieDeviceType,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PcieDeviceType {
    PvScsi,
    VirtioNet {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mac_address: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rx_queue_size: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tx_queue_size: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vhost: Option<bool>,
    },
    StandardGpu,
    VirtioGpu,
    Passthrough {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        host: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        multifunction: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rombar: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        romfile: Option<String>,
    },
    PassthroughGpu {
        resource: String,
    },
    Ich9IntelHda {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        codec: Option<String>,
    },
    IvshmemPlain {
        resource: String,
    },
}

impl From<&PcieDeviceType> for Arc<dyn PcieDeviceApi> {
    fn from(device: &PcieDeviceType) -> Self {
        match device {
            PcieDeviceType::PvScsi => Arc::new(super::PvScsiController::default()),
            PcieDeviceType::VirtioNet {
                mac_address,
                rx_queue_size,
                tx_queue_size,
                vhost,
                ..
            } => Arc::new(super::VirtioNetController::new(
                None,
                mac_address.clone(),
                *rx_queue_size,
                *tx_queue_size,
                *vhost,
            )),
            PcieDeviceType::StandardGpu => Arc::new(StandardGpuController::default()),
            PcieDeviceType::VirtioGpu => Arc::new(VirtioGpuController::default()),
            PcieDeviceType::Passthrough {
                resource,
                host,
                id,
                multifunction,
                rombar,
                romfile,
            } => Arc::new(PassthroughPcieController::new(
                host.clone()
                    .unwrap_or_else(|| resource.clone().unwrap_or_default()),
                id.clone(),
                *multifunction,
                *rombar,
                romfile.clone(),
            )),
            PcieDeviceType::PassthroughGpu { resource } => {
                Arc::new(PassthroughGpuController::new(resource.clone()))
            }
            PcieDeviceType::Ich9IntelHda { codec } => {
                Arc::new(Ich9IntelHdaController::new(codec.clone()))
            }
            PcieDeviceType::IvshmemPlain { resource } => {
                Arc::new(IvshmemPlainController::new(resource.clone()))
            }
        }
    }
}

pub trait PcieDeviceApi: Display {
    fn as_any(&self) -> &dyn Any;

    fn device_kind(&self) -> PcieDeviceType;

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct PassthroughPcieController {
    host: String,
    id: Option<String>,
    multifunction: Option<bool>,
    rombar: Option<bool>,
    romfile: Option<String>,
}

impl PassthroughPcieController {
    pub fn new(
        host: String,
        id: Option<String>,
        multifunction: Option<bool>,
        rombar: Option<bool>,
        romfile: Option<String>,
    ) -> Self {
        Self {
            host,
            id,
            multifunction,
            rombar,
            romfile,
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn multifunction(&self) -> Option<bool> {
        self.multifunction
    }

    pub fn rombar(&self) -> Option<bool> {
        self.rombar
    }

    pub fn romfile(&self) -> Option<&str> {
        self.romfile.as_deref()
    }
}

impl Display for PassthroughPcieController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PCIe Passthrough ({})", self.host)
    }
}

impl PcieDeviceApi for PassthroughPcieController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::Passthrough {
            resource: None,
            host: Some(self.host.clone()),
            id: self.id.clone(),
            multifunction: self.multifunction,
            rombar: self.rombar,
            romfile: self.romfile.clone(),
        }
    }
}

// GPU Device Controllers

#[derive(Debug, Clone, Default)]
pub struct StandardGpuController {}

impl Display for StandardGpuController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Standard GPU")
    }
}

impl PcieDeviceApi for StandardGpuController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::StandardGpu
    }
}

#[derive(Debug, Clone, Default)]
pub struct VirtioGpuController {}

impl Display for VirtioGpuController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Virtio GPU")
    }
}

impl PcieDeviceApi for VirtioGpuController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::VirtioGpu
    }
}

#[derive(Debug, Clone)]
pub struct PassthroughGpuController {
    resource: String,
}

impl PassthroughGpuController {
    pub fn new(resource: String) -> Self {
        Self { resource }
    }

    pub fn resource(&self) -> &str {
        &self.resource
    }
}

impl Display for PassthroughGpuController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Passthrough GPU ({})", self.resource)
    }
}

impl PcieDeviceApi for PassthroughGpuController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::PassthroughGpu {
            resource: self.resource.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ich9IntelHdaController {
    codec: Option<String>,
}

impl Ich9IntelHdaController {
    pub fn new(codec: Option<String>) -> Self {
        Self { codec }
    }

    pub fn codec(&self) -> Option<&str> {
        self.codec.as_deref()
    }
}

impl Display for Ich9IntelHdaController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Intel HDA Audio Controller")
    }
}

impl PcieDeviceApi for Ich9IntelHdaController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::Ich9IntelHda {
            codec: self.codec.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IvshmemPlainController {
    resource: String,
}

impl IvshmemPlainController {
    pub fn new(resource: String) -> Self {
        Self { resource }
    }

    pub fn resource(&self) -> &str {
        &self.resource
    }
}

impl Display for IvshmemPlainController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IVSHMEM Plain ({})", self.resource)
    }
}

impl PcieDeviceApi for IvshmemPlainController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::IvshmemPlain {
            resource: self.resource.clone(),
        }
    }
}

pub trait PcieControllerApi: Display {
    fn register_pcie_device(
        &self,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: Option<PcieAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<PcieAddress, Arc<dyn PcieDeviceApi>>;
}
