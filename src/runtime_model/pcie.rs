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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcieDeviceKind {
    PvScsi,
    VirtioNet,
    StandardGpu,
    VirtioGpu,
    Passthrough,
    PassthroughGpu,
    Ich9IntelHda,
    IvshmemPlain,
}

#[derive(Debug, Clone)]
pub struct PciePassthroughSpec {
    pub resource: Option<String>,
    pub host: String,
    pub id: Option<String>,
    pub multifunction: Option<bool>,
    pub rombar: Option<bool>,
    pub romfile: Option<String>,
}

impl From<&PcieDeviceType> for Arc<dyn PcieDeviceApi> {
    fn from(device: &PcieDeviceType) -> Self {
        match device {
            PcieDeviceType::PvScsi => Arc::new(super::PvScsiController::default()),
            PcieDeviceType::VirtioNet { .. } => Arc::new(super::VirtioNetController::default()),
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
                resource.clone(),
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
    fn device_kind(&self) -> PcieDeviceKind;
    fn network_resource(&self) -> Option<&crate::runtime_model::NetworkResource> {
        None
    }
    fn resource_id(&self) -> Option<&str> {
        None
    }

    fn passthrough_spec(&self) -> Option<PciePassthroughSpec> {
        None
    }

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
    fn qemu_args(&self, bus: &PcieBus, address: PcieAddress) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub struct PassthroughPcieController {
    resource: Option<String>,
    host: String,
    id: Option<String>,
    multifunction: Option<bool>,
    rombar: Option<bool>,
    romfile: Option<String>,
}

impl PassthroughPcieController {
    pub fn new(
        resource: Option<String>,
        host: String,
        id: Option<String>,
        multifunction: Option<bool>,
        rombar: Option<bool>,
        romfile: Option<String>,
    ) -> Self {
        Self {
            resource,
            host,
            id,
            multifunction,
            rombar,
            romfile,
        }
    }
}

impl Display for PassthroughPcieController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PCIe Passthrough ({})", self.host)
    }
}

impl PcieDeviceApi for PassthroughPcieController {
    fn device_kind(&self) -> PcieDeviceKind {
        PcieDeviceKind::Passthrough
    }

    fn passthrough_spec(&self) -> Option<PciePassthroughSpec> {
        Some(PciePassthroughSpec {
            resource: self.resource.clone(),
            host: self.host.clone(),
            id: self.id.clone(),
            multifunction: self.multifunction,
            rombar: self.rombar,
            romfile: self.romfile.clone(),
        })
    }

    fn qemu_args(&self, _bus: &PcieBus, address: PcieAddress) -> Vec<String> {
        let mut device = format!("vfio-pci,host={}", self.host);
        if let Some(id) = &self.id {
            device.push_str(&format!(",id={id}"));
        }
        if let Some(multifunction) = self.multifunction {
            device.push_str(&format!(
                ",multifunction={}",
                if multifunction { 1 } else { 0 }
            ));
        }
        if let Some(rombar) = self.rombar {
            device.push_str(&format!(",rombar={}", if rombar { 1 } else { 0 }));
        }
        if let Some(romfile) = &self.romfile {
            device.push_str(&format!(",romfile={romfile}"));
        }

        let addr = if address.function() == 0 {
            format!("0x{:x}", address.device())
        } else {
            format!("0x{:x}.{}", address.device(), address.function())
        };
        device.push_str(&format!(",bus=pcie.0,addr={addr}"));

        vec!["-device".to_string(), device]
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
