use derive_getters::Getters;
use derive_new::new;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcieBusDeviceKind {
    VirtioNet,
    ScsiController,
    VirtioScsiSingleDisk,
    HostPci,
    Ivshmem,
}

#[allow(dead_code)]
pub trait PcieDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn device_kind(&self) -> PcieBusDeviceKind;
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Getters, new)]
pub struct PcieAddress {
    device: u8,
    function: u8,
}

impl std::fmt::Display for PcieAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.device, self.function)
    }
}
