use crate::runtime::{IdeDevice, SataDevice, scsi::ScsiDevice};

#[allow(dead_code)]
pub trait StorageDevice {
    fn storage_options(&self) -> StorageOptions;
}

#[allow(dead_code)]
pub struct StorageOptions {
    pub device_type: StorageDeviceType,
    pub read_only: bool,
    pub cache_mode: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageDeviceType {
    Hdd,
    Ssd,
    Odd,
}

#[derive(Debug)]
pub struct Ssd { pub resource: String }
impl Ssd {
    pub fn new(resource: String) -> Self {
        Ssd { resource }
    }
}

#[derive(Debug)]
pub struct Hdd { pub resource: String }
impl Hdd {
    pub fn new(resource: String) -> Self {
        Hdd { resource }
    }
}

#[derive(Debug)]
pub struct Cdrom { pub resource: String }
impl Cdrom {
    pub fn new(resource: String) -> Self {
        Cdrom { resource }
    }
}

impl IdeDevice for Ssd {}

impl SataDevice for Ssd {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ScsiDevice for Ssd {
    fn as_any(&self) -> &dyn std::any::Any {
        self
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

impl IdeDevice for Hdd {}
impl SataDevice for Hdd {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl ScsiDevice for Hdd {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl StorageDevice for Hdd {
    fn storage_options(&self) -> StorageOptions {
        StorageOptions {
            device_type: StorageDeviceType::Hdd,
            read_only: false,
            cache_mode: None,
        }
    }
}

impl IdeDevice for Cdrom {}
impl SataDevice for Cdrom {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl ScsiDevice for Cdrom {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl StorageDevice for Cdrom {
    fn storage_options(&self) -> StorageOptions {
        StorageOptions {
            device_type: StorageDeviceType::Odd,
            read_only: true,
            cache_mode: None,
        }
    }
}
