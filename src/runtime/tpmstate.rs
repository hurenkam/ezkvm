use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{RootDevice, RootDeviceKind};

/// Note: `size` field from Proxmox conf is invariant and carries no information beyond
/// `version = "v2.0"`. It is not stored; Phase 7 emits it as a constant.
#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct TpmState {
    storage_volume: String,
    version: String,
}

impl RootDevice for TpmState {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "tpmstate" }
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::TpmState }
}
