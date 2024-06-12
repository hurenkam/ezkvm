use serde::Deserialize;
use crate::yaml::QemuArgs;

#[derive(Debug,Deserialize)]
pub struct Display {
    driver: String,
    gl: bool
}

impl QemuArgs for Display {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let gl = if self.gl { ",gl=on" } else { "" };
        vec![
            format!("-display {}{}",self.driver,gl),
            "-audiodev pipewire,id=audiodev0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=audiodev0".to_string(),
        ]
    }
}
