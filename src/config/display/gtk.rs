use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::display::Display;

#[derive(Deserialize,Debug,Clone)]
pub struct Gtk {
    gl: bool,
}

impl QemuDevice for Gtk {
    fn get_args(&self, index: usize) -> Vec<String> {
        let gl = if self.gl { ",gl=on" } else { "" };
        vec![
            format!("-display gtk{}",gl),
            "-audiodev pipewire,id=audiodev0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=audiodev0".to_string(),
        ]
    }
}

#[typetag::deserialize(name = "gtk")]
impl Display for Gtk {}
