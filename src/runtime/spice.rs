use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{RootDevice, RootDeviceKind};

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct SpiceDisplay {
    port: Option<u16>,
    addr: Option<String>,
    disable_ticketing: bool,
    gl: bool,
    rendernode: Option<String>,
    clipboard: bool,
}

impl RootDevice for SpiceDisplay {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "spice_display" }
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::SpiceDisplay }
}
