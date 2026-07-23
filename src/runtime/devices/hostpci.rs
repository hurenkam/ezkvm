use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{PcieDevice, pcie::PcieBusDeviceKind};

/// ⚠️ MULTI-FUNCTION: Store ONE HostPci per Proxmox `hostpciN:` line. Never split into two
/// structs for .0 and .1 — use `functions: Vec<u8>` to hold both function numbers.
#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct HostPci {
    base_bdf: String,
    functions: Vec<u8>,
    pcie: bool,
    x_vga: bool,
    rombar: Option<bool>,
    romfile: Option<String>,
}

impl PcieDevice for HostPci {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn device_kind(&self) -> PcieBusDeviceKind { PcieBusDeviceKind::HostPci }
}
