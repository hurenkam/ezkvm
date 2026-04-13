use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};

/// Serial configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    /// Serial type (pty, file, socket, stdio)
    pub r#type: String,

    /// Port number (for multi-port setups)
    #[serde(default)]
    pub port: Option<u32>,

    /// Output file path for `file` serial backends
    pub path: Option<String>,

    /// Hostname or IP for `socket` serial backends
    pub host: Option<String>,

    /// TCP port for `socket` serial backends
    pub socket_port: Option<u16>,

    /// Whether the socket backend should listen in server mode
    #[serde(default = "default_true")]
    pub server: bool,

    /// Whether QEMU should wait for a socket connection before continuing
    #[serde(default)]
    pub wait: bool,
}

fn default_true() -> bool {
    true
}

impl From<SerialConfig> for QemuArgs {
    fn from(serial: SerialConfig) -> Self {
        let mut args = QemuArgs::new();

        match serial.r#type.as_str() {
            "pty" => {
                args.push_str("-serial");
                args.push("pty".to_string());
            }
            "stdio" => {
                args.push_str("-serial");
                args.push("stdio".to_string());
            }
            "file" => {
                args.push_str("-serial");
                args.push(format!(
                    "file:{}",
                    serial.path.unwrap_or_else(|| "/dev/null".to_string())
                ));
            }
            "socket" => {
                args.push_str("-serial");
                let host = serial.host.unwrap_or_else(|| "127.0.0.1".to_string());
                let port = serial.socket_port.unwrap_or(4444);
                let mut spec = format!("tcp:{}:{}", host, port);
                if serial.server {
                    spec.push_str(",server");
                }
                if !serial.wait {
                    spec.push_str(",nowait");
                }
                args.push(spec);
            }
            _ => {
                // Default to pty
                args.push_str("-serial");
                args.push("pty".to_string());
            }
        }

        args
    }
}
