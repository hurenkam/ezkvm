use crate::config::{default_when_missing, QemuDevice};
use derive_getters::Getters;
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum VNCSocket {
    TcpPort { addr: String, port: u16 },
    UnixSocket { path: String },
}
impl Default for VNCSocket {
    fn default() -> Self {
        VNCSocket::TcpPort {
            addr: "127.0.0.1".to_string(),
            port: 5900,
        }
    }
}

#[derive(Debug, PartialEq, Default, Deserialize, Getters)]
#[serde(default)]
pub struct VNC {
    #[serde(default, flatten, deserialize_with = "default_when_missing")]
    socket: VNCSocket,
}

impl VNC {
    pub fn new_with_address_and_port(addr: String, port: u16) -> Self {
        Self {
            socket: VNCSocket::TcpPort { addr, port },
        }
    }

    pub fn new_with_socket(path: String) -> Self {
        Self {
            socket: VNCSocket::UnixSocket { path },
        }
    }
}

impl QemuDevice for VNC {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        match self.socket {
            VNCSocket::TcpPort { ref addr, ref port } => {
                result.extend(vec![
                    format!("-vnc {}:{}", addr, port),
                    "-display vnc=:0".to_string(),
                ]);
            }

            VNCSocket::UnixSocket { ref path } => {
                result.extend(vec![format!("-vnc unix:{}", path)]);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VNC;
    #[test]
    fn test_defaults() {
        let input = r#""#;

        let data = VNC {
            socket: Default::default(),
        };

        let output: Vec<String> = vec![
            "-vnc 127.0.0.1:5900".to_string(),
            "-display vnc=:0".to_string(),
        ];

        assert_eq!(serde_yaml::from_str::<VNC>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_unix_socket() {
        let input = r#"
            path: /var/ezkvm/unix.socket
        "#;

        let data = VNC {
            socket: VNCSocket::UnixSocket {
                path: "/var/ezkvm/unix.socket".to_string(),
            },
        };

        let output: Vec<String> = vec!["-vnc unix:/var/ezkvm/unix.socket".to_string()];

        assert_eq!(serde_yaml::from_str::<VNC>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }
}
