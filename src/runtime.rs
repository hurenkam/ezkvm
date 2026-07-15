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
pub use devices::PvScsi;
pub use ide::{IdeAddress, IdeDevice};
pub use memory::Memory;
pub use pci::PciDevice;
pub use pcie::{PcieAddress, PcieDevice};
pub use q35::Q35Chipset;
pub use sata::{SataAddress, SataDevice};
pub use scsi::ScsiAddress;
pub use storage::{Ssd, StorageDevice};

use derive_getters::Getters;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Mutex;
use std::{any::TypeId, sync::Arc};

use crate::runtime::scsi::ScsiDevice;

#[derive(Debug, Default, Clone)]
pub struct BusDeviceRegistry(pub HashMap<TypeId, HashMap<u8, Vec<Arc<dyn BusDevice>>>>);

#[allow(dead_code)]
impl BusDeviceRegistry {
    pub fn add_bus(&mut self, device_type: TypeId) -> u8 {
        //println!("Adding bus for device type: {:?}", device_type);
        self.0.entry(device_type).or_default();
        let registry = self.0.get_mut(&device_type).expect("Registry should exist");
        let bus_id = registry.len() as u8;
        registry.insert(bus_id, Vec::new());
        bus_id
    }
    pub fn add_bus_device(&mut self, bus_type: TypeId, bus_id: u8, device: Arc<dyn BusDevice>) {
        println!(
            "Adding device for bus id: {}, type id: {:?}, device name: {}",
            bus_id,
            bus_type,
            device.get_name(),
        );
        let registry = self.0.get_mut(&bus_type).expect("Registry should exist");
        let bus = registry.get_mut(&bus_id).expect("Bus should exist");
        bus.push(device);
    }

    pub fn get_pcie_bus(&self, bus_id: u8) -> Option<&Vec<Arc<dyn BusDevice>>> {
        let registry = self.0.get(&TypeId::of::<dyn PcieDevice>())?;
        registry.get(&bus_id)
    }
    pub fn get_pci_bus(&self, bus_id: u8) -> Option<&Vec<Arc<dyn BusDevice>>> {
        let registry = self.0.get(&TypeId::of::<dyn PciDevice>())?;
        registry.get(&bus_id)
    }
    pub fn get_sata_bus(&self, bus_id: u8) -> Option<&Vec<Arc<dyn BusDevice>>> {
        let registry = self.0.get(&TypeId::of::<dyn SataDevice>())?;
        registry.get(&bus_id)
    }
    //pub fn get_usb_bus(&self, bus_id: u8) -> Option<&Vec<Arc<dyn BusDevice>>> {
    //    let registry = self.0.get(&TypeId::of::<dyn UsbDevice>())?;
    //    registry.get(&bus_id)
    //}
}

#[allow(dead_code)]
#[derive(Debug, Default, Getters)]
pub struct Runtime {
    root_devices: Vec<Arc<dyn RootDevice>>,
    bus_devices: BusDeviceRegistry,
}

#[allow(dead_code)]
impl Runtime {
    pub fn new() -> Self {
        Runtime {
            root_devices: Vec::new(),
            bus_devices: BusDeviceRegistry(HashMap::new()),
        }
    }

    pub fn register_root_device(&mut self, device: Arc<dyn RootDevice>) {
        self.root_devices.push(device);
    }

    pub fn register_bus(&mut self, device_type: TypeId) -> u8 {
        self.bus_devices.add_bus(device_type)
    }

    pub fn register_bus_device(&mut self, bus_id: u8, device: Arc<dyn BusDevice>) {
        let device_type = device.get_type();
        let registry = self
            .bus_devices
            .0
            .get_mut(&device_type)
            .expect("Registry should exist");
        let bus = registry.get_mut(&bus_id).expect("Bus should exist");
        bus.push(device);
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
    bus_devices: Arc<Mutex<BusDeviceRegistry>>,
}
#[allow(dead_code)]
impl RuntimeBuilder {
    pub fn new() -> Self {
        RuntimeBuilder {
            root_devices: Mutex::new(Vec::new()),
            bus_devices: Arc::new(Mutex::new(BusDeviceRegistry(HashMap::new()))),
        }
    }

    pub fn bus_devices(&self) -> Arc<Mutex<BusDeviceRegistry>> {
        Arc::clone(&self.bus_devices)
    }

    pub fn build(self) -> Result<Runtime, ()> {
        let root_devices = self
            .root_devices
            .into_inner()
            .expect("Root devices mutex should not be poisoned");
        let bus_devices = self.bus_devices.lock().unwrap();
        Ok(Runtime {
            root_devices,
            bus_devices: bus_devices.clone(),
        })
    }

    pub fn with_memory(&self, memory: Memory) -> &Self {
        self.root_devices.lock().unwrap().push(Arc::new(memory));
        self
    }

    pub fn with_chipset(&self, chipset: Chipset) -> &Self {
        self.root_devices.lock().unwrap().push(Arc::new(chipset));
        self
    }

    pub fn with_pcie_device(&self, device: Arc<dyn PcieDevice>, address: PcieAddress) {
        let mut bus_devices = self.bus_devices.lock().unwrap();
        device.set_pcie_address(address);
        let bus_id = address.get_bus_id();
        bus_devices.add_bus_device(TypeId::of::<dyn PcieDevice>(), bus_id, device);
    }

    pub fn with_sata_device(&self, device: Arc<dyn SataDevice>, address: SataAddress) {
        let mut bus_devices = self.bus_devices.lock().unwrap();
        device.set_sata_address(address);
        let sata_bus_id = address.get_bus_id();
        bus_devices.add_bus_device(TypeId::of::<dyn SataDevice>(), sata_bus_id, device);
    }

    pub fn with_scsi_device(&self, device: Arc<dyn ScsiDevice>, address: ScsiAddress) {
        let mut bus_devices = self.bus_devices.lock().unwrap();
        device.set_scsi_address(address);
        let scsi_bus_id = address.get_bus_id();
        bus_devices.add_bus_device(TypeId::of::<dyn ScsiDevice>(), scsi_bus_id, device);
    }
}

#[allow(dead_code)]
pub trait BusDevice: Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn get_name(&self) -> &str;
    fn get_type(&self) -> TypeId;
}

#[allow(dead_code)]
pub trait BusAddress: Debug + Send + Sync + 'static {
    fn get_bus_id(&self) -> u8;
}
