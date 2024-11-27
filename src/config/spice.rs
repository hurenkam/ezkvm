use crate::config::QemuDevice;
use derive_getters::Getters;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Default, Deserialize)]
#[serde(untagged)]
pub enum SpiceSocket {
    TcpPort { addr: String, port: u16 },
    UnixSocket { path: String },
    #[default]
    None
}

#[derive(Debug, Default, Deserialize)]
#[serde(tag="gl")]
pub enum SpiceDisplay {
    #[default]
    #[serde(rename = "off")]
    Disabled,
    #[serde(rename = "on")]
    Enabled { render_node: Option<String> }
}

#[derive(Debug, Deserialize, Getters)]
pub struct Spice {
    #[serde(flatten)]
    socket: SpiceSocket,
    #[serde(default)]
    display: SpiceDisplay
}

impl Spice {
    pub fn new_with_address_and_port(addr: String, port: u16) -> Self {
        Self { socket: SpiceSocket::TcpPort { addr, port }, display: SpiceDisplay::Disabled }
    }

    pub fn new_with_address_port_and_render_node(addr: String, port: u16, render_node: Option<String>) -> Self {
        Self { socket: SpiceSocket::TcpPort { addr, port }, display: SpiceDisplay::Enabled {render_node: render_node } }
    }

    pub fn new_with_socket_and_render_node(path: String, render_node: Option<String>) -> Self {
        Self { socket: SpiceSocket::UnixSocket { path }, display: SpiceDisplay::Enabled {render_node: render_node } }
    }
}

impl QemuDevice for Spice {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        match self.socket {
            SpiceSocket::TcpPort { ref addr, ref port } => {
                result.extend(vec![
                    format!(
                        "-spice port={},addr={},disable-ticketing=on",
                        port, addr
                    ),
                ]);
                match &self.display {
                    SpiceDisplay::Disabled => {}
                    SpiceDisplay::Enabled { render_node } => {
                        let render_node = match render_node {
                            Some(render_node) => format!(",rendernode={}",render_node),
                            None => "".to_string()
                        };
                        result.extend(vec![
                            format!(
                                "--display egl-headless{}", render_node
                            ),
                        ]);
                    }
                }
            },

            SpiceSocket::UnixSocket { ref path } => {
                let gl_options = match &self.display {
                    SpiceDisplay::Disabled => "".to_string(),
                    SpiceDisplay::Enabled { render_node } => {
                        let render_node = match render_node {
                            Some(render_node) => format!(",rendernode={}",render_node),
                            None => "".to_string()
                        };
                        format!(",gl=on{}",render_node)
                    }
                };

                result.extend(vec![
                    format!("-spice unix=on,addr={}{},disable-ticketing=on",path,gl_options)
                ]);
            }

            SpiceSocket::None => {}
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
