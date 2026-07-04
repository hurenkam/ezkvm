use std::{fmt, sync::Arc};

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

pub trait GuestAgentApi: fmt::Display {
    fn config(&self) -> &GuestAgent;
}

pub struct GuestAgentModelBuilder {}

impl GuestAgentModelBuilder {
    pub fn build(guest_agent: GuestAgent) -> Arc<dyn GuestAgentApi> {
        Arc::new(GuestAgentModel {
            config: guest_agent,
        })
    }
}

struct GuestAgentModel {
    config: GuestAgent,
}

impl GuestAgentApi for GuestAgentModel {
    fn config(&self) -> &GuestAgent {
        &self.config
    }
}

impl fmt::Display for GuestAgentModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.config.enabled {
            write!(f, "Guest Agent (enabled)")
        } else {
            write!(f, "Guest Agent (disabled)")
        }
    }
}
