use derive_getters::Getters;
use derive_new::new;

use crate::runtime::UsbDevice;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsbDeviceKind {
    NetworkController,
    Tablet,
    HostPassthrough { resource: String },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct GenericUsbDevice {
    kind: UsbDeviceKind,
}

impl UsbDevice for GenericUsbDevice {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::UsbBusDeviceKind {
        crate::runtime::UsbBusDeviceKind::Generic
    }
}
