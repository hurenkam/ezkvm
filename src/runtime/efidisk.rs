use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{RootDevice, RootDeviceKind};

/// ⚠️ DUAL-SIZE: `logical_size` ("4M") and `block_device_size_bytes` (540672) are structurally
/// unrelated. 4 MiB ≠ 540672 bytes. Never convert between them.
#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct EfiDisk {
    storage_volume: String,
    efitype: Option<String>,
    pre_enrolled_keys: bool,
    ms_cert: Option<String>,
    logical_size: String,
    block_device_size_bytes: Option<u64>,
}

impl RootDevice for EfiDisk {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "efidisk" }
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::EfiDisk }
}
