use super::{default_when_missing, QemuDevice};
use derive_getters::Getters;
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum SpiceSocket {
    TcpPort { addr: String, port: u16 },
    UnixSocket { path: String },
}
impl Default for SpiceSocket {
    fn default() -> Self {
        SpiceSocket::TcpPort {
            addr: "127.0.0.1".to_string(),
            port: 5900,
        }
    }
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(tag = "gl")]
pub enum SpiceDisplay {
    #[default]
    #[serde(rename = "off")]
    Disabled,
    #[serde(rename = "on")]
    Enabled { render_node: Option<String> },
}

#[derive(Debug, PartialEq, Default, Deserialize, Getters)]
#[serde(default)]
pub struct Spice {
    #[serde(default, flatten, deserialize_with = "default_when_missing")]
    socket: SpiceSocket,
    #[serde(default, flatten, deserialize_with = "default_when_missing")]
    display: SpiceDisplay,
    #[serde(default = "Spice::disable_ticketing_default")]
    disable_ticketing: bool,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    tls_port: Option<u16>,
    #[serde(default)]
    x509_dir: Option<String>,
    #[serde(default)]
    websocket_port: Option<u16>,
}

impl Spice {
    fn disable_ticketing_default() -> bool {
        true
    }

    pub fn new_with_address_and_port(addr: String, port: u16) -> Self {
        Self {
            socket: SpiceSocket::TcpPort { addr, port },
            display: SpiceDisplay::Disabled,
            disable_ticketing: Self::disable_ticketing_default(),
            password: None,
            tls_port: None,
            x509_dir: None,
            websocket_port: None,
        }
    }

    pub fn new_with_address_port_and_render_node(
        addr: String,
        port: u16,
        render_node: Option<String>,
    ) -> Self {
        Self {
            socket: SpiceSocket::TcpPort { addr, port },
            display: SpiceDisplay::Enabled { render_node },
            disable_ticketing: Self::disable_ticketing_default(),
            password: None,
            tls_port: None,
            x509_dir: None,
            websocket_port: None,
        }
    }

    pub fn new_with_socket_and_render_node(path: String, render_node: Option<String>) -> Self {
        Self {
            socket: SpiceSocket::UnixSocket { path },
            display: SpiceDisplay::Enabled { render_node },
            disable_ticketing: Self::disable_ticketing_default(),
            password: None,
            tls_port: None,
            x509_dir: None,
            websocket_port: None,
        }
    }

    fn get_security_options(&self) -> String {
        let mut options: Vec<String> = vec![];

        if self.disable_ticketing {
            options.push("disable-ticketing=on".to_string());
        }

        if let Some(ref password) = self.password {
            options.push(format!("password={}", password));
        }

        if let Some(tls_port) = self.tls_port {
            options.push(format!("tls-port={}", tls_port));
        }

        if let Some(ref x509_dir) = self.x509_dir {
            options.push(format!("x509-dir={}", x509_dir));
        }

        if let Some(websocket_port) = self.websocket_port {
            options.push(format!("websocket={}", websocket_port));
        }

        if options.is_empty() {
            "".to_string()
        } else {
            format!(",{}", options.join(","))
        }
    }
}

impl QemuDevice for Spice {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        match self.socket {
            SpiceSocket::TcpPort { ref addr, ref port } => {
                let security = self.get_security_options();
                result.extend(vec![format!(
                    "-spice port={},addr={}{}",
                    port, addr, security
                )]);
                match &self.display {
                    SpiceDisplay::Disabled => {}
                    SpiceDisplay::Enabled { render_node } => {
                        let render_node = match render_node {
                            Some(render_node) => format!(",rendernode={}", render_node),
                            None => "".to_string(),
                        };
                        result.extend(vec![format!("-display egl-headless{}", render_node)]);
                    }
                }
            }

            SpiceSocket::UnixSocket { ref path } => {
                let gl_options = match &self.display {
                    SpiceDisplay::Disabled => "".to_string(),
                    SpiceDisplay::Enabled { render_node } => {
                        let render_node = match render_node {
                            Some(render_node) => format!(",rendernode={}", render_node),
                            None => "".to_string(),
                        };
                        format!(",gl=on{}", render_node)
                    }
                };

                let security = self.get_security_options();

                result.extend(vec![format!(
                    "-spice unix=on,addr={}{}{}",
                    path, gl_options, security
                )]);
            }
        }

        result.extend(vec![
            "-device virtio-serial-pci".to_string(),
            "-chardev spicevmc,id=vdagent,name=vdagent".to_string(),
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string(),
            "-audiodev spice,id=spice-backend0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0"
                .to_string(),
        ]);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::Spice;
    use super::*;
    #[test]
    fn test_defaults() {
        let input = r#""#;

        let data = Spice {
            socket: Default::default(),
            display: Default::default(),
            disable_ticketing: true,
            password: None,
            tls_port: None,
            x509_dir: None,
            websocket_port: None,
        };

        let output: Vec<String> = vec![
            "-spice port=5900,addr=127.0.0.1,disable-ticketing=on".to_string(),
            "-device virtio-serial-pci".to_string(),
            "-chardev spicevmc,id=vdagent,name=vdagent".to_string(),
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string(),
            "-audiodev spice,id=spice-backend0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0"
                .to_string(),
        ];

        assert_eq!(serde_yaml::from_str::<Spice>(input).unwrap(), data);
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

        let data = Spice {
            socket: SpiceSocket::TcpPort {
                addr: "127.0.0.1".to_string(),
                port: 5900,
            },
            display: SpiceDisplay::Enabled {
                render_node: Some("/dev/dri/renderD128".to_string()),
            },
            disable_ticketing: true,
            password: None,
            tls_port: None,
            x509_dir: None,
            websocket_port: None,
        };

        let output: Vec<String> = vec![
            "-spice port=5900,addr=127.0.0.1,disable-ticketing=on".to_string(),
            "-display egl-headless,rendernode=/dev/dri/renderD128".to_string(),
            "-device virtio-serial-pci".to_string(),
            "-chardev spicevmc,id=vdagent,name=vdagent".to_string(),
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string(),
            "-audiodev spice,id=spice-backend0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0"
                .to_string(),
        ];

        assert_eq!(serde_yaml::from_str::<Spice>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_unix_socket_with_gl() {
        let input = r#"
            path: /var/ezkvm/unix.socket
            gl: on
            render_node: /dev/dri/renderD128
        "#;

        let data = Spice {
            socket: SpiceSocket::UnixSocket {
                path: "/var/ezkvm/unix.socket".to_string(),
            },
            display: SpiceDisplay::Enabled {
                render_node: Some("/dev/dri/renderD128".to_string()),
            },
            disable_ticketing: true,
            password: None,
            tls_port: None,
            x509_dir: None,
            websocket_port: None,
        };

        let output: Vec<String> = vec![
            "-spice unix=on,addr=/var/ezkvm/unix.socket,gl=on,rendernode=/dev/dri/renderD128,disable-ticketing=on".to_string(),
            "-device virtio-serial-pci".to_string(),
            "-chardev spicevmc,id=vdagent,name=vdagent".to_string(),
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string(),
            "-audiodev spice,id=spice-backend0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0".to_string()
        ];

        assert_eq!(serde_yaml::from_str::<Spice>(input).unwrap(), data);
        assert_eq!(data.get_qemu_args(0), output);
    }

    #[test]
    fn test_security_and_transport_options() {
        let input = r#"
            addr: 0.0.0.0
            port: 5901
            disable_ticketing: false
            password: secret
            tls_port: 5902
            x509_dir: /etc/pki/qemu
            websocket_port: 6100
        "#;

        let spice: Spice = serde_yaml::from_str(input).unwrap();
        let args = spice.get_qemu_args(0);
        assert_eq!(
            args[0],
            "-spice port=5901,addr=0.0.0.0,password=secret,tls-port=5902,x509-dir=/etc/pki/qemu,websocket=6100"
        );
    }
}
