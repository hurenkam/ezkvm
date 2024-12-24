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

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(tag = "gl")]
pub enum VNCDisplay {
    #[default]
    #[serde(rename = "off")]
    Disabled,
    #[serde(rename = "on")]
    Enabled { render_node: Option<String> },
}

#[derive(Debug, PartialEq, Default, Deserialize, Getters)]
#[serde(default)]
pub struct VNC {
    #[serde(default, flatten, deserialize_with = "default_when_missing")]
    socket: VNCSocket,
    #[serde(default, flatten, deserialize_with = "default_when_missing")]
    display: VNCDisplay,
}

impl VNC {
    pub fn new_with_address_and_port(addr: String, port: u16) -> Self {
        Self {
            socket: VNCSocket::TcpPort { addr, port },
            display: VNCDisplay::Disabled,
        }
    }

    pub fn new_with_address_port_and_render_node(
        addr: String,
        port: u16,
        render_node: Option<String>,
    ) -> Self {
        Self {
            socket: VNCSocket::TcpPort { addr, port },
            display: VNCDisplay::Enabled {
                render_node: render_node,
            },
        }
    }

    pub fn new_with_socket_and_render_node(path: String, render_node: Option<String>) -> Self {
        Self {
            socket: VNCSocket::UnixSocket { path },
            display: VNCDisplay::Enabled {
                render_node: render_node,
            },
        }
    }
}

impl QemuDevice for VNC {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        match self.socket {
            VNCSocket::TcpPort { ref addr, ref port } => {
                result.extend(vec![format!("-vnc port={},addr={}", port, addr)]);
                match &self.display {
                    VNCDisplay::Disabled => {}
                    VNCDisplay::Enabled { render_node } => {
                        let render_node = match render_node {
                            Some(render_node) => format!(",rendernode={}", render_node),
                            None => "".to_string(),
                        };
                        result.extend(vec![format!("-display egl-headless{}", render_node)]);
                    }
                }
            }

            VNCSocket::UnixSocket { ref path } => {
                //let gl_options = match &self.display {
                //    VNCDisplay::Disabled => "".to_string(),
                //    VNCDisplay::Enabled { render_node } => {
                //        let render_node = match render_node {
                //            Some(render_node) => format!(",rendernode={}", render_node),
                //            None => "".to_string(),
                //        };
                //        format!(",gl=on{}", render_node)
                //    }
                //};

                //result.extend(vec![format!("-vnc unix:{}{}", path, gl_options)]);
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
            display: Default::default(),
        };

        let output: Vec<String> = vec!["-vnc port=5900,addr=127.0.0.1".to_string()];

        assert_eq!(serde_yaml::from_str::<VNC>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_tcp_port_with_gl() {
        let input = r#"
            addr: 127.0.0.1
            port: 5900
            gl: on
            render_node: /dev/dri/renderD128
        "#;

        let data = VNC {
            socket: VNCSocket::TcpPort {
                addr: "127.0.0.1".to_string(),
                port: 5900,
            },
            display: VNCDisplay::Enabled {
                render_node: Some("/dev/dri/renderD128".to_string()),
            },
        };

        let output: Vec<String> = vec![
            "-vnc port=5900,addr=127.0.0.1".to_string(),
            "-display egl-headless,rendernode=/dev/dri/renderD128".to_string(),
        ];

        assert_eq!(serde_yaml::from_str::<VNC>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_unix_socket_with_gl() {
        let input = r#"
            path: /var/ezkvm/unix.socket
            gl: on
            render_node: /dev/dri/renderD128
        "#;

        let data = VNC {
            socket: VNCSocket::UnixSocket {
                path: "/var/ezkvm/unix.socket".to_string(),
            },
            display: VNCDisplay::Enabled {
                render_node: Some("/dev/dri/renderD128".to_string()),
            },
        };

        let output: Vec<String> = vec![
            "-vnc unix=on,addr=/var/ezkvm/unix.socket,gl=on,rendernode=/dev/dri/renderD128"
                .to_string(),
        ];

        assert_eq!(serde_yaml::from_str::<VNC>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }
}
