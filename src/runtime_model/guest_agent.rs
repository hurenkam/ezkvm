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
    fn qemu_args(&self, vm_name: &str) -> Vec<String>;
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

    fn qemu_args(&self, vm_name: &str) -> Vec<String> {
        if !self.config.enabled {
            return vec![];
        }
        let socket_path = format!("/var/run/ezkvm/{vm_name}.agent");
        vec![
            "-chardev".to_string(),
            format!("socket,path={socket_path},server=on,wait=off,id=qga0"),
            "-device".to_string(),
            "virtio-serial".to_string(),
            "-device".to_string(),
            "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0".to_string(),
        ]
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
