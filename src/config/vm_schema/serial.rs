use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};

/// Serial device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialDeviceConfig {
    /// Serial backend type (socket, file, chardev, pty, stdio)
    pub r#type: String,

    /// Optional explicit chardev identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Port number (for multi-port setups)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,

    /// Output file path for `file` backends or socket path for unix sockets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Hostname or IP for `socket` serial backends
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    /// TCP port for `socket` serial backends
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket_port: Option<u16>,

    /// Whether the socket backend should listen in server mode
    #[serde(default = "default_true")]
    pub server: bool,

    /// Whether QEMU should wait for a socket connection before continuing
    #[serde(default)]
    pub wait: bool,

    /// Raw chardev backend specification used when `type: chardev`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chardev: Option<String>,
}

pub type SerialConfig = SerialDeviceConfig;

fn default_true() -> bool {
    true
}

impl From<SerialDeviceConfig> for QemuArgs {
    fn from(serial: SerialDeviceConfig) -> Self {
        let mut args = QemuArgs::new();
        let fallback_id = serial
            .port
            .map(|port| format!("serial{}", port))
            .unwrap_or_else(|| "serial0".to_string());
        let chardev_id = serial.id.clone().unwrap_or(fallback_id);
        let mut serial_device_spec = format!("isa-serial,chardev={}", chardev_id);
        if let Some(port) = serial.port {
            serial_device_spec.push_str(&format!(",index={}", port));
        }

        match serial.r#type.as_str() {
            "pty" => {
                args.push_str("-chardev");
                args.push(format!("pty,id={}", chardev_id));
            }
            "stdio" => {
                args.push_str("-chardev");
                args.push(format!("stdio,id={}", chardev_id));
            }
            "file" => {
                let path = serial.path.unwrap_or_else(|| "/dev/null".to_string());
                args.push_str("-chardev");
                args.push(format!("file,id={},path={}", chardev_id, path));
            }
            "socket" => {
                let mut spec = if let Some(path) = serial.path {
                    format!("socket,id={},path={}", chardev_id, path)
                } else {
                    let host = serial.host.unwrap_or_else(|| "127.0.0.1".to_string());
                    let port = serial.socket_port.unwrap_or(4444);
                    format!("socket,id={},host={},port={}", chardev_id, host, port)
                };
                if serial.server {
                    spec.push_str(",server=on");
                } else {
                    spec.push_str(",server=off");
                }
                if serial.wait {
                    spec.push_str(",wait=on");
                } else {
                    spec.push_str(",wait=off");
                }
                args.push_str("-chardev");
                args.push(spec);
            }
            "chardev" => {
                let mut spec = serial.chardev.unwrap_or_else(|| "null".to_string());
                if !spec.contains("id=") {
                    spec.push_str(&format!(",id={}", chardev_id));
                }
                args.push_str("-chardev");
                args.push(spec);
            }
            _ => {
                args.push_str("-chardev");
                args.push(format!("pty,id={}", chardev_id));
            }
        }

        args.push_str("-device");
        args.push(serial_device_spec);

        args
    }
}
