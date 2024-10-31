use crate::config::system::bios::Bios;
use crate::config::types::QemuDevice;
use serde::Deserialize;

const OVMF_BIOS_FILE: &str = "/usr/share/ezkvm/OVMF_CODE.secboot.4m.fd";
const BOOT_SPLASH_FILE: &str = "/usr/share/ezkvm/bootsplash.jpg";

#[derive(Deserialize, Debug, Clone)]
pub struct OVMF {
    file: String,
    uuid: String,
}

impl OVMF {
    #[cfg(test)]
    pub fn new(file: String, uuid: String) -> Self {
        Self { file, uuid }
    }
}

impl QemuDevice for OVMF {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![
            format!(
                "-boot menu=on,strict=on,reboot-timeout=1000,splash={}",
                BOOT_SPLASH_FILE
            ),
            format!("-smbios type=1,uuid={}", self.uuid.clone()),
            format!(
                "-drive if=pflash,unit=0,format=raw,readonly=on,file={}",
                OVMF_BIOS_FILE
            ),
            format!(
                "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file={},size=540672",
                self.file.clone()
            ),
        ]
    }
}

#[typetag::deserialize(name = "ovmf")]
impl Bios for OVMF {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let ovmf = OVMF {
            file: "the_file".to_string(),
            uuid: "the_uuid".to_string(),
        };
        assert_eq!(ovmf.get_qemu_args(0), vec![
            "-boot menu=on,strict=on,reboot-timeout=1000,splash=/usr/share/ezkvm/bootsplash.jpg".to_string(),
            "-smbios type=1,uuid=the_uuid".to_string(),
            "-drive if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE.secboot.4m.fd".to_string(),
            "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=the_file,size=540672".to_string()
        ]);
    }
}
