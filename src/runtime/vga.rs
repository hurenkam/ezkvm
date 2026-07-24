use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{RootDevice, RootDeviceKind};

/// Proxmox `vga` field. Only `mode == "none"` is currently handled by the QEMU emitter
/// (per ROADMAP/RESEARCH scope) — other mode values are out of scope for this phase.
#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct VgaConfig {
    mode: String,
}

impl RootDevice for VgaConfig {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "vga_config" }
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::VgaConfig }
}
