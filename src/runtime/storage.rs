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
pub enum StorageDeviceType {
    Hdd,
    Ssd,
    Odd,
}

#[derive(Debug)]
pub struct Ssd {}
impl Ssd {
    pub fn new() -> Self {
        Ssd {}
    }
}

impl IdeDevice for Ssd {}
impl SataDevice for Ssd {}
impl ScsiDevice for Ssd {}

impl StorageDevice for Ssd {
    fn storage_options(&self) -> StorageOptions {
        StorageOptions {
            device_type: StorageDeviceType::Ssd,
            read_only: false,
            cache_mode: None,
        }
    }
}
