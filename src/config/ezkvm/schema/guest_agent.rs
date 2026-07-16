use serde::{Deserialize, Serialize};

/// Config-level guest agent configuration for the virtual machine.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuestAgent {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for GuestAgent {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_true() -> bool {
    true
}
