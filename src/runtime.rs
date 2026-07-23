mod chipset;
mod devices;
mod efidisk;
mod tpmstate;
mod audio;
mod spice;
mod rawargs;
mod ide;
mod isa;
mod memory;
mod pci;
mod pcie;
mod q35;
mod sata;
mod scsi;
mod storage;
mod usb;

pub use chipset::Chipset;
pub use devices::{GenericPciDevice, GenericUsbDevice, HostPci, Ivshmem, PciDeviceKind, PvScsi, PvScsiBuilder, UsbDeviceKind, VirtioNetPcie};
pub use efidisk::EfiDisk;
pub use tpmstate::TpmState;
pub use audio::AudioDevice;
pub use spice::SpiceDisplay;
pub use rawargs::RawArgs;
pub use ide::{IdeAddress, IdeDevice};
pub use memory::Memory;
pub use pci::{PciAddress, PciDevice, PciBusDeviceKind};
pub use pcie::{PcieAddress, PcieDevice, PcieBusDeviceKind};
pub use q35::{Q35Chipset, Q35ChipsetBuilder};
pub use sata::{SataAddress, SataDevice};
pub use scsi::{ScsiAddress, ScsiDevice};
pub use storage::{Cdrom, Hdd, Ssd, StorageDevice, StorageDeviceType};
pub use usb::{UsbAddress, UsbDevice, UsbBusDeviceKind};
pub use isa::IsaBusDeviceKind;

use derive_getters::Getters;
use std::fmt::Debug;
use std::{any::TypeId, sync::Arc};

#[allow(dead_code)]
#[derive(Debug, Default, Getters)]
pub struct Runtime {
    root_devices: Vec<Arc<dyn RootDevice>>,
}

impl std::fmt::Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Runtime:")?;
        for device in &self.root_devices {
            format_root_device(f, device.as_ref())?;
        }
        Ok(())
    }
}

fn format_root_device(f: &mut std::fmt::Formatter<'_>, device: &dyn RootDevice) -> std::fmt::Result {
    match device.device_kind() {
        RootDeviceKind::Memory => {
            let memory = device.as_any().downcast_ref::<Memory>().unwrap();
            writeln!(f, "  {}", memory)
        }
        RootDeviceKind::Chipset => {
            let chipset = device.as_any().downcast_ref::<Chipset>().unwrap();
            let rendered = format!("{}", chipset).replace('\n', "\n  ");
            writeln!(f, "  {}", rendered)
        }
        RootDeviceKind::EfiDisk => {
            let efidisk = device.as_any().downcast_ref::<EfiDisk>().unwrap();
            writeln!(f, "  EfiDisk(volume={:?}, logical_size={:?})", efidisk.storage_volume(), efidisk.logical_size())
        }
        _ => writeln!(f, "  {}()", device.get_name()),
    }
}

#[allow(dead_code)]
impl Runtime {
    pub fn new() -> Self {
        Runtime {
            root_devices: Vec::new(),
        }
    }

    pub fn register_root_device(&mut self, device: Arc<dyn RootDevice>) {
        self.root_devices.push(device);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootDeviceKind {
    Memory,
    Chipset,
    EfiDisk,
    TpmState,
    AudioDevice,
    SpiceDisplay,
    RawArgs,
}

#[allow(dead_code)]
pub trait RootDevice: Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn get_name(&self) -> &str;
    fn device_kind(&self) -> RootDeviceKind;
    fn get_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

#[allow(dead_code)]
pub struct RuntimeBuilder {
    root_devices: Vec<Arc<dyn RootDevice>>,
}
#[allow(dead_code)]
impl RuntimeBuilder {
    pub fn new() -> Self {
        RuntimeBuilder {
            root_devices: Vec::new(),
        }
    }

    pub fn build(self) -> Result<Runtime, ()> {
        Ok(Runtime { root_devices: self.root_devices })
    }

    pub fn with_memory(mut self, memory: Memory) -> Self {
        self.root_devices.push(Arc::new(memory));
        self
    }

    pub fn with_chipset(mut self, chipset: Chipset) -> Self {
        self.root_devices.push(Arc::new(chipset));
        self
    }

    pub fn with_efidisk(mut self, efidisk: EfiDisk) -> Self {
        self.root_devices.push(Arc::new(efidisk));
        self
    }

    pub fn with_tpmstate(mut self, tpmstate: TpmState) -> Self {
        self.root_devices.push(Arc::new(tpmstate));
        self
    }

    pub fn with_audio_device(mut self, audio: AudioDevice) -> Self {
        self.root_devices.push(Arc::new(audio));
        self
    }

    pub fn with_spice_display(mut self, spice: SpiceDisplay) -> Self {
        self.root_devices.push(Arc::new(spice));
        self
    }

    pub fn with_raw_args(mut self, raw_args: RawArgs) -> Self {
        self.root_devices.push(Arc::new(raw_args));
        self
    }
}
