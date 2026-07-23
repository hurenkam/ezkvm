use std::fmt::Debug;

use derive_getters::Getters;
use derive_new::new;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciBusDeviceKind {
    GenericPci,
    PvScsi,
}

#[allow(dead_code)]
pub trait PciDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn device_kind(&self) -> PciBusDeviceKind;
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Getters, new)]
pub struct PciAddress {
    device: u8,
    function: u8,
}

impl std::fmt::Display for PciAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.device, self.function)
    }
}
