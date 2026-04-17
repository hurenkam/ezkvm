use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

/// Apple SMC emulation settings used by macOS guests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleSmcConfig {
    /// Enable Apple SMC device emission.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,

    /// Apple OSK key required by isa-applesmc.
    pub osk: String,
}
