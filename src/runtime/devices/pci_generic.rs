use derive_getters::Getters;
use derive_new::new;

use crate::runtime::PciDevice;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciDeviceKind {
    NetworkController,
    QxlGpu,
    Ac97,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct GenericPciDevice {
    kind: PciDeviceKind,
}

impl PciDevice for GenericPciDevice {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::PciBusDeviceKind {
        crate::runtime::PciBusDeviceKind::GenericPci
    }
}
