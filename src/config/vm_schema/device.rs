use serde::{Deserialize, Serialize};

use super::super::{AudioDeviceConfig, InputDeviceConfig};
use super::{DisplayConfig, DriveConfig, NetworkConfig, SerialConfig};

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    /// Storage devices
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drives: Vec<DriveConfig>,

    /// Network devices
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<NetworkConfig>,

    /// Display devices
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub displays: Vec<DisplayConfig>,

    /// Serial devices
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serials: Vec<SerialConfig>,

    /// Explicit input devices
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<InputDeviceConfig>,

    /// Audio devices backed by the selected audio backend
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audio: Vec<AudioDeviceConfig>,
}
