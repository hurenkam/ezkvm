use derive_getters::Getters;
use derive_new::new;
use std::fmt::Debug;

use crate::runtime::StorageDevice;

use crate::runtime::StorageDeviceType;

#[allow(dead_code)]
pub trait SataDevice: StorageDevice + Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn device_kind(&self) -> StorageDeviceType {
        self.storage_options().device_type
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Getters, new)]
pub struct SataAddress {
    port: u8,
    device: u8,
}

impl std::fmt::Display for SataAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.port, self.device)
    }
}
