#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsaBusDeviceKind {
    PvScsi,
}

#[allow(dead_code)]
pub trait IsaDevice {
    fn as_any(&self) -> &dyn std::any::Any;
    fn device_kind(&self) -> IsaBusDeviceKind;
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct IsaAddress(u16);
