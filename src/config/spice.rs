use crate::config::QemuDevice;
use derive_getters::Getters;
use serde::Deserialize;

#[derive(Debug, Deserialize, Getters)]
pub struct Spice {
    addr: String,
    port: u16,
}

impl Spice {
    pub fn new(addr: String, port: u16) -> Self {
        Self { addr, port }
    }
}

impl QemuDevice for Spice {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![
            format!(
                "-spice port={},addr={},disable-ticketing=on",
                self.port, self.addr
            ),
            "-device virtio-serial-pci".to_string(),
            "-chardev spicevmc,id=vdagent,name=vdagent".to_string(),
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string(),
            "-audiodev spice,id=spice-backend0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0"
                .to_string(),
        ]
    }
}
