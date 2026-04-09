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
    #[serde(default)]
    password: Option<bool>,
    #[serde(default)]
    websocket_port: Option<u16>,
    #[serde(default)]
    sasl: Option<bool>,
}

impl Vnc {
    pub fn new_with_address_and_port(addr: String, port: u16) -> Self {
        Self {
            socket: VncSocket::TcpPort { addr, port },
            password: None,
            websocket_port: None,
            sasl: None,
        }
    }

    pub fn new_with_socket(path: String) -> Self {
        Self {
            socket: VncSocket::UnixSocket { path },
            password: None,
            websocket_port: None,
            sasl: None,
        }
    }

    fn build_options_suffix(&self) -> String {
        let mut options: Vec<String> = vec![];

        if self.password == Some(true) {
            options.push("password=on".to_string());
        }
        if let Some(port) = self.websocket_port {
            options.push(format!("websocket={}", port));
        }
        if self.sasl == Some(true) {
            options.push("sasl=on".to_string());
        }

        if options.is_empty() {
            "".to_string()
        } else {
            format!(",{}", options.join(","))
        }
    }
}

impl QemuDevice for Vnc {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        match self.socket {
            VncSocket::TcpPort { ref addr, ref port } => {
                // QEMU expects a VNC display number, where TCP port 5900 maps to display 0.
                let display = if *port >= 5900 { port - 5900 } else { *port };
                result.push(format!(
                    "-vnc {}:{}{}",
                    addr,
                    display,
                    self.build_options_suffix()
                ));
            }

            VncSocket::UnixSocket { ref path } => {
                result.extend(vec![format!(
                    "-vnc unix:{}{}",
                    path,
                    self.build_options_suffix()
                )]);
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
            password: None,
            websocket_port: None,
            sasl: None,
        };

        let output: Vec<String> = vec!["-vnc 127.0.0.1:0".to_string()];

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
            password: None,
            websocket_port: None,
            sasl: None,
        };

        let output: Vec<String> = vec!["-vnc unix:/var/ezkvm/unix.socket".to_string()];

        assert_eq!(serde_yaml::from_str::<Vnc>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_tcp_port_conversion() {
        let data = Vnc::new_with_address_and_port("0.0.0.0".to_string(), 5903);
        let output: Vec<String> = vec!["-vnc 0.0.0.0:3".to_string()];

        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_security_and_transport_options() {
        let input = r#"
            addr: 0.0.0.0
            port: 5901
            password: true
            websocket_port: 6101
            sasl: true
        "#;

        let data: Vnc = serde_yaml::from_str(input).unwrap();
        let output: Vec<String> = vec!["-vnc 0.0.0.0:1,password=on,websocket=6101,sasl=on".to_string()];

        assert_eq!(data.get_qemu_args(0), output);
    }
}
