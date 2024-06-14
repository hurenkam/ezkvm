use serde::Deserialize;
use crate::config::bios::Bios;
use crate::config::config::QemuDevice;

const BOOT_SPLASH_FILE: &str = "/mnt/usr/share/qemu-server/bootsplash.jpg";
const OVMF_BIOS_FILE: &str = "/mnt/usr/share/pve-edk2-firmware//OVMF_CODE_4M.secboot.fd";

#[derive(Deserialize,Debug,Clone)]
pub struct OVMF {
    file: String,
    uuid: String,
}

impl QemuDevice for OVMF {
    fn get_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-boot menu=on,strict=on,reboot-timeout=1000,splash={}",BOOT_SPLASH_FILE),
            format!("-smbios type=1,uuid={}",self.uuid.clone()),
            format!("-drive if=pflash,unit=0,format=raw,readonly=on,file={}",OVMF_BIOS_FILE),
            format!("-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file={},size=540672",self.file.clone()),
        ]
    }
}

#[typetag::deserialize(name = "ovmf")]
impl Bios for OVMF {}
