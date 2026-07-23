use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{PcieDevice, pcie::PcieBusDeviceKind};

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct Ivshmem {
    id: String,
    mem_path: String,
    size: String,
}

impl PcieDevice for Ivshmem {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn device_kind(&self) -> PcieBusDeviceKind { PcieBusDeviceKind::Ivshmem }
}
