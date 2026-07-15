use std::sync::Mutex;

use crate::runtime::{BusDevice, IdeAddress, SataAddress, SataDevice, ScsiAddress, scsi::ScsiDevice};

#[allow(dead_code)]
pub trait StorageDevice: BusDevice {
    fn storage_options(&self) -> StorageOptions;
}

#[allow(dead_code)]
pub struct StorageOptions {
    pub device_type: StorageDeviceType,
    pub read_only: bool,
    pub cache_mode: Option<String>,
}

#[allow(dead_code)]
pub enum StorageDeviceType {
    Hdd,
    Ssd,
    Odd,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum StorageAddress {
    Scsi(ScsiAddress),
    Sata(SataAddress),
    Ide(IdeAddress),
}

#[derive(Debug)]
pub struct Ssd {
    address: Mutex<Option<StorageAddress>>,
}
impl Ssd {
    pub fn new() -> Self {
        Ssd {
            address: Mutex::new(None),
        }
    }
}
impl SataDevice for Ssd {
    fn get_sata_address(&self) -> Option<SataAddress> {
        match *self.address.lock().unwrap() {
            Some(StorageAddress::Sata(addr)) => Some(addr),
            _ => None,
        }
    }
    fn set_sata_address(&self, address: SataAddress) {
        println!("Setting SATA address to {:?}", address);
        let mut addr = self.address.lock().unwrap();
        *addr = Some(StorageAddress::Sata(address));
    }
}
impl ScsiDevice for Ssd {
    fn get_scsi_address(&self) -> Option<ScsiAddress> {
        match *self.address.lock().unwrap() {
            Some(StorageAddress::Scsi(addr)) => Some(addr),
            _ => None,
        }
    }
    fn set_scsi_address(&self, address: ScsiAddress) {
        println!("Setting SCSI address to {:?}", address);
        let mut addr = self.address.lock().unwrap();
        *addr = Some(StorageAddress::Scsi(address));
    }
}
impl StorageDevice for Ssd {
    fn storage_options(&self) -> StorageOptions {
        StorageOptions {
            device_type: StorageDeviceType::Ssd,
            read_only: false,
            cache_mode: None,
        }
    }
}
impl BusDevice for Ssd {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_name(&self) -> &str {
        "ssd"
    }

    fn get_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
}
