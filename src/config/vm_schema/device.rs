use serde::{Deserialize, Serialize};

use super::{DisplayConfig, DriveConfig, NetworkConfig, SerialConfig};

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    /// Storage devices
    #[serde(default)]
    pub drives: Vec<DriveConfig>,

    /// Network devices
    #[serde(default)]
    pub networks: Vec<NetworkConfig>,

    /// Display devices
    #[serde(default)]
    pub displays: Vec<DisplayConfig>,

    /// Serial devices
    #[serde(default)]
    pub serials: Vec<SerialConfig>,
}
