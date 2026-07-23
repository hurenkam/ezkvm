use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{RootDevice, RootDeviceKind};

/// Note: Bus placement is fixed by Q35 conventions; codec devices are derived from
/// `device_type` at emit time, not stored in the Runtime.
#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct AudioDevice {
    device_type: String,
    driver: String,
}

impl RootDevice for AudioDevice {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "audio_device" }
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::AudioDevice }
}
