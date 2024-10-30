use crate::config::display::Display;
use crate::config::gpu::Gpu;
use crate::config::qemu_device::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Gtk {
    gl: bool,
}
impl Gtk {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self { gl: true })
    }
}

impl QemuDevice for Gtk {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let gl = if self.gl { ",gl=on" } else { "" };
        vec![
            format!("-display gtk{}", gl),
            "-audiodev pipewire,id=audiodev0".to_string(),
            "-device usb-tablet".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=audiodev0"
                .to_string(),
        ]
    }
}

#[typetag::deserialize(name = "gtk")]
impl Display for Gtk {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let display = Gtk { gl: true };
        let expected: Vec<String> = vec![
            "-display gtk,gl=on".to_string(),
            "-audiodev pipewire,id=audiodev0".to_string(),
            "-device usb-tablet".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=audiodev0"
                .to_string(),
        ];
        assert_eq!(display.get_qemu_args(0), expected);
    }
}
