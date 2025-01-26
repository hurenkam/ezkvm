use super::{default_when_missing, QemuDevice};
use derive_getters::Getters;
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum VncSocket {
    TcpPort { addr: String, port: u16 },
    UnixSocket { path: String },
}
impl Default for VncSocket {
    fn default() -> Self {
        VncSocket::TcpPort {
            addr: "127.0.0.1".to_string(),
            port: 5900,
        }
    }
}

#[derive(Debug, PartialEq, Default, Deserialize, Getters)]
#[serde(default)]
pub struct Vnc {
    #[serde(default, flatten, deserialize_with = "default_when_missing")]
    socket: VncSocket,
}

impl Vnc {
    pub fn new_with_address_and_port(addr: String, port: u16) -> Self {
        Self {
            socket: VncSocket::TcpPort { addr, port },
        }
    }

    pub fn new_with_socket(path: String) -> Self {
        Self {
            socket: VncSocket::UnixSocket { path },
        }
    }
}

impl QemuDevice for Vnc {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        match self.socket {
            VncSocket::TcpPort { ref addr, ref port } => {
                result.extend(vec![
                    format!("-vnc {}:{}", addr, port),
                    "-display vnc=:0".to_string(),
                ]);
            }

            VncSocket::UnixSocket { ref path } => {
                result.extend(vec![format!("-vnc unix:{}", path)]);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::Vnc;
    use super::*;
    #[test]
    fn test_defaults() {
        let input = r#""#;

        let data = Vnc {
            socket: Default::default(),
        };

        let output: Vec<String> = vec![
            "-vnc 127.0.0.1:5900".to_string(),
            "-display vnc=:0".to_string(),
        ];

        assert_eq!(serde_yaml::from_str::<Vnc>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_unix_socket() {
        let input = r#"
            path: /var/ezkvm/unix.socket
        "#;

        let data = Vnc {
            socket: VncSocket::UnixSocket {
                path: "/var/ezkvm/unix.socket".to_string(),
            },
        };

        let output: Vec<String> = vec!["-vnc unix:/var/ezkvm/unix.socket".to_string()];

        assert_eq!(serde_yaml::from_str::<Vnc>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }
}
