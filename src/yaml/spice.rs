use crate::yaml::{LgClientArgs, QemuArgs};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Spice {
    port: u16,
    addr: String,
}

impl QemuArgs for Spice {
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

// looking-glass-client app:shmFile=/dev/kvmfr0 spice:host=0.0.0.0 spice:port=5903 input:escapeKey=KEY_F12 input:grabKeyboard win:size=1707x1067 win:fullscreen
impl LgClientArgs for Spice {
    fn get_lg_client_args(&self, _index: usize) -> Vec<String> {
        vec![
            format!("spice:host={}", self.addr),
            format!("spice:port={}", self.port),
        ]
    }
}
