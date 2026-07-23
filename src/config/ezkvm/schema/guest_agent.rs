use serde::{Deserialize, Serialize};

/// Config-level guest agent configuration for the virtual machine.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuestAgentSchema {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for GuestAgentSchema {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_true() -> bool {
    true
}
