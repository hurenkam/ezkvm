use crate::config::system::bios::Bios;
use crate::config::types::QemuDevice;
use crate::{optional_value_getter, required_value_getter};
use paste::paste;
use serde::{Deserialize, Serialize};

const OVMF64_2M_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE.fd";
const OVMF64_2M_SECURE_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE.secboot.fd";
const OVMF64_4M_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE_4M.fd";
const OVMF64_4M_SECURE_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE_4M.secboot.fd";
const OVMF32_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF32_CODE_4M.fd";

#[derive(Deserialize, Serialize, Default, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OvmfArch {
    #[default]
    #[serde(rename = "64bit")]
    Arch64,
    #[serde(rename = "32bit")]
    Arch32,
}
#[derive(Deserialize, Serialize, Default, Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(usize)]
pub enum OvmfSize {
    #[serde(rename = "2M")]
    Size2M = 0,
    #[default]
    #[serde(rename = "4M")]
    Size4M = 1,
}
#[allow(dead_code)]
const OVMF_ROM_SIZE: [usize; 2] = [1966080, 3653632];
#[allow(dead_code)]
const OVMF_VAR_SIZE: [usize; 2] = [131072, 540672];

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Ovmf {
    #[serde(default = "Ovmf::settings_file_default", rename = "file")]
    settings_file: String,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    arch: Option<OvmfArch>,
    #[serde(default)]
    size: Option<OvmfSize>,
    #[serde(default = "Ovmf::secure_boot_default")]
    secure_boot: Option<bool>,
}

impl Ovmf {
    required_value_getter!(settings_file("file"): String = "NO_SETTINGS_FILE_PROVIDED".to_string());
    optional_value_getter!(uuid("uuid"): String);

    pub fn secure_boot_default() -> Option<bool> {
        Some(true)
    }

    #[cfg(test)]
    pub fn new(
        settings_file: String,
        uuid: Option<String>,
        arch: Option<OvmfArch>,
        size: Option<OvmfSize>,
        secure_boot: Option<bool>,
    ) -> Self {
        Self {
            settings_file,
            uuid,
            arch,
            size,
            secure_boot,
        }
    }

    fn boot_rom_file(&self) -> String {
        let arch = self.arch.unwrap_or_default();
        let result = match arch {
            OvmfArch::Arch32 => OVMF32_BOOT_ROM,
            OvmfArch::Arch64 => {
                let size = self.size.unwrap_or_default();
                let secure_boot = self.secure_boot.unwrap_or_default();

                if size == OvmfSize::Size4M {
                    if secure_boot {
                        OVMF64_4M_SECURE_BOOT_ROM
                    } else {
                        OVMF64_4M_BOOT_ROM
                    }
                } else if secure_boot {
                    OVMF64_2M_SECURE_BOOT_ROM
                } else {
                    OVMF64_2M_BOOT_ROM
                }
            }
        };

        format!(",file={}", result)
    }
    fn boot_rom_size(&self) -> String {
        "".to_string()
    }
    fn settings_size(&self) -> String {
        match &self.size {
            None => format!(
                ",size={}",
                OVMF_VAR_SIZE.get(OvmfSize::Size4M as usize).unwrap()
            ),
            Some(value) => format!(",size={}", OVMF_VAR_SIZE.get(*value as usize).unwrap()),
        }
    }
}
impl QemuDevice for Ovmf {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![
            "-boot menu=on,strict=on,reboot-timeout=1000".to_string(),
            format!("-smbios type=1{}", self.uuid()),
            format!(
                "-drive if=pflash,unit=0,format=raw,readonly=on{}{}",
                self.boot_rom_file(),
                self.boot_rom_size()
            ),
            format!(
                "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw{}{}",
                self.settings_file(),
                self.settings_size()
            ),
        ]
    }
}
#[typetag::deserialize(name = "ovmf")]
impl Bios for Ovmf {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let actual: Ovmf = serde_yaml::from_str(
            r#"
            "#,
        )
        .unwrap();
        let converted = serde_yaml::to_string(&actual).unwrap();
        println!("{}", converted);
        let ovmf = Ovmf {
            settings_file: Ovmf::settings_file_default(),
            uuid: None,
            arch: None,
            size: None,
            secure_boot: Ovmf::secure_boot_default(),
        };
        assert_eq!(actual, ovmf);

        assert_eq!(ovmf.get_qemu_args(0), vec![
            "-boot menu=on,strict=on,reboot-timeout=1000".to_string(),
            "-smbios type=1".to_string(),
            "-drive if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE_4M.secboot.fd".to_string(),
            "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=NO_SETTINGS_FILE_PROVIDED,size=540672".to_string()
        ]);
    }
    #[test]
    fn test_valid() {
        let actual: Ovmf = serde_yaml::from_str(
            r#"
                    file: "the_file"
                    uuid: "the_uuid"
                    arch: "64bit"
                    size: "2M"
                    secure_boot: false
                "#,
        )
        .unwrap();
        let ovmf = Ovmf {
            settings_file: "the_file".to_string(),
            uuid: Some("the_uuid".to_string()),
            arch: Some(OvmfArch::Arch64),
            size: Some(OvmfSize::Size2M),
            secure_boot: Some(false),
        };
        assert_eq!(actual, ovmf);

        assert_eq!(
            ovmf.get_qemu_args(0),
            vec![
                "-boot menu=on,strict=on,reboot-timeout=1000".to_string(),
                "-smbios type=1,uuid=the_uuid".to_string(),
                "-drive if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/ezkvm/OVMF_CODE.fd"
                    .to_string(),
                "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=the_file,size=131072"
                    .to_string()
            ]
        );
    }
}
