use std::fmt::Debug;
use serde::Deserialize;
use crate::config::config::QemuDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct Spice {
    port: u32,
    addr: String
}

impl QemuDevice for Spice {
    fn get_args(&self, _index: usize) -> Vec<String> {
        vec![
            format!("-spice port={},addr={},disable-ticketing=on",self.port,self.addr),
            "-device virtio-serial-pci".to_string(),
            "-chardev spicevmc,id=vdagent,name=vdagent".to_string(),
            "-device virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string(),
            "-audiodev spice,id=spice-backend0".to_string(),
            "-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc".to_string(),
            "-device hda-duplex,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0".to_string(),
        ]
    }
}