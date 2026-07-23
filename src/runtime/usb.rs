use derive_getters::Getters;
use derive_new::new;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbBusDeviceKind {
    Generic,
}

#[allow(dead_code)]
pub trait UsbDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn device_kind(&self) -> UsbBusDeviceKind;
}

#[allow(dead_code)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Getters, new)]
pub struct UsbAddress {
    port: String,
}
