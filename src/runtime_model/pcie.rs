use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, sync::Arc};

use super::ControllerApi;
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
#[derive(Debug, Clone, Deserialize, Serialize, Getters)]
pub struct PcieDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<PcieBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
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
    },
    StandardGpu,
    VirtioGpu,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcieDeviceKind {
    PvScsi,
    VirtioNet,
    StandardGpu,
    VirtioGpu,
    PassthroughGpu,
    Ich9IntelHda,
    IvshmemPlain,
}
impl From<&PcieDeviceType> for Arc<dyn PcieDeviceApi> {
    fn from(device: &PcieDeviceType) -> Self {
        match device {
            PcieDeviceType::PvScsi => Arc::new(super::PvScsiController::default()),
            PcieDeviceType::VirtioNet { .. } => Arc::new(super::VirtioNetController::default()),
            PcieDeviceType::StandardGpu => Arc::new(StandardGpuController::default()),
            PcieDeviceType::VirtioGpu => Arc::new(VirtioGpuController::default()),
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
    fn device_kind(&self) -> PcieDeviceKind;
    fn network_resource(&self) -> Option<&crate::runtime_model::NetworkResource> {
        None
    }
    fn resource_id(&self) -> Option<&str> {
        None
    }

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
    fn qemu_args(&self, bus: &PcieBus, address: PcieAddress) -> Vec<String>;
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
    fn device_kind(&self) -> PcieDeviceKind {
        PcieDeviceKind::StandardGpu
    }

    fn qemu_args(&self, _bus: &PcieBus, _address: PcieAddress) -> Vec<String> {
        vec!["-device".to_string(), "VGA".to_string()]
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
    fn device_kind(&self) -> PcieDeviceKind {
        PcieDeviceKind::VirtioGpu
    }

    fn qemu_args(&self, _bus: &PcieBus, _address: PcieAddress) -> Vec<String> {
        vec!["-device".to_string(), "virtio-gpu-pci".to_string()]
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
}

impl Display for PassthroughGpuController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Passthrough GPU ({})", self.resource)
    }
}

impl PcieDeviceApi for PassthroughGpuController {
    fn device_kind(&self) -> PcieDeviceKind {
        PcieDeviceKind::PassthroughGpu
    }

    fn resource_id(&self) -> Option<&str> {
        Some(&self.resource)
    }

    fn qemu_args(&self, _bus: &PcieBus, _address: PcieAddress) -> Vec<String> {
        vec!["-device".to_string(), "vfio-pci".to_string()]
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
}

impl Display for Ich9IntelHdaController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Intel HDA Audio Controller")
    }
}

impl PcieDeviceApi for Ich9IntelHdaController {
    fn device_kind(&self) -> PcieDeviceKind {
        PcieDeviceKind::Ich9IntelHda
    }

    fn qemu_args(&self, _bus: &PcieBus, _address: PcieAddress) -> Vec<String> {
        let mut args = vec!["-device".to_string(), "ich9-intel-hda".to_string()];
        if let Some(codec) = &self.codec {
            args.push("-device".to_string());
            args.push(codec.clone());
        }
        args
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
}

impl Display for IvshmemPlainController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IVSHMEM Plain ({})", self.resource)
    }
}

impl PcieDeviceApi for IvshmemPlainController {
    fn device_kind(&self) -> PcieDeviceKind {
        PcieDeviceKind::IvshmemPlain
    }

    fn resource_id(&self) -> Option<&str> {
        Some(&self.resource)
    }

    fn qemu_args(&self, _bus: &PcieBus, _address: PcieAddress) -> Vec<String> {
        vec!["-device".to_string(), "ivshmem-plain".to_string()]
    }
}

pub trait PcieControllerApi: ControllerApi + Display {
    fn register_pcie_device(
        &self,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: Option<PcieAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<PcieAddress, Arc<dyn PcieDeviceApi>>;
}
