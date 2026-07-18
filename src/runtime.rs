mod chipset;
mod devices;
mod ide;
mod isa;
mod memory;
mod pci;
mod pcie;
mod q35;
mod sata;
mod scsi;
mod storage;

pub use chipset::Chipset;
pub use devices::PvScsiBuilder;
pub use ide::{IdeAddress, IdeDevice};
pub use memory::Memory;
pub use pci::PciDevice;
pub use pcie::{PcieAddress, PcieDevice};
pub use q35::{Q35Chipset, Q35ChipsetBuilder};
pub use sata::{SataAddress, SataDevice};
pub use scsi::ScsiAddress;
pub use storage::{Ssd, StorageDevice};

use derive_getters::Getters;
use std::fmt::Debug;
use std::sync::Mutex;
use std::{any::TypeId, sync::Arc};

#[allow(dead_code)]
#[derive(Debug, Default, Getters)]
pub struct Runtime {
    root_devices: Vec<Arc<dyn RootDevice>>,
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

#[allow(dead_code)]
pub trait RootDevice: Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn get_name(&self) -> &str;
    fn get_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

#[allow(dead_code)]
pub struct RuntimeBuilder {
    root_devices: Mutex<Vec<Arc<dyn RootDevice>>>,
}
#[allow(dead_code)]
impl RuntimeBuilder {
    pub fn new() -> Self {
        RuntimeBuilder {
            root_devices: Mutex::new(Vec::new()),
        }
    }

    pub fn build(self) -> Result<Runtime, ()> {
        let root_devices = self
            .root_devices
            .into_inner()
            .expect("Root devices mutex should not be poisoned");
        Ok(Runtime { root_devices })
    }

    pub fn with_memory(self, memory: Memory) -> Self {
        self.root_devices.lock().unwrap().push(Arc::new(memory));
        self
    }

    pub fn with_chipset(self, chipset: Chipset) -> Self {
        self.root_devices.lock().unwrap().push(Arc::new(chipset));
        self
    }
}
