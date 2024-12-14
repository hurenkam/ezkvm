use crate::config::system::bios::Bios;
use crate::config::types::QemuDevice;
use crate::{optional_value_getter, required_value_getter};
use paste::paste;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

const OVMF64_2M_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE.fd";
const OVMF64_2M_SECURE_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE.secboot.fd";
const OVMF64_4M_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE_4M.fd";
const OVMF64_4M_SECURE_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF_CODE_4M.secboot.fd";
const OVMF32_BOOT_ROM: &str = "/usr/share/ezkvm/OVMF32_CODE_4M.fd";

#[derive(Deserialize, Serialize, Default, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OVMFArch {
    #[default]
    #[serde(rename = "64bit")]
    Arch64,
    #[serde(rename = "32bit")]
    Arch32,
}
#[derive(Deserialize, Serialize, Default, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OVMFSize {
    #[default]
    #[serde(rename = "4M")]
    Size4M,
    #[serde(rename = "2M")]
    Size2M,
}
static OVMF_SIZE: LazyLock<HashMap<OVMFSize, (u64, u64)>> = LazyLock::new(|| {
    HashMap::from([
        // type                 rom  settings
        (OVMFSize::Size2M, (1966080, 131072)),
        (OVMFSize::Size4M, (3653632, 540672)),
    ])
});

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct OVMF {
    #[serde(default = "OVMF::settings_file_default", rename = "file")]
    settings_file: String,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    arch: Option<OVMFArch>,
    #[serde(default)]
    size: Option<OVMFSize>,
    #[serde(default = "OVMF::secure_boot_default")]
    secure_boot: Option<bool>,
}

impl OVMF {
    required_value_getter!(settings_file("file"): String = "NO_SETTINGS_FILE_PROVIDED".to_string());
    optional_value_getter!(uuid("uuid"): String);

    pub fn secure_boot_default() -> Option<bool> {
        Some(true)
    }

    #[cfg(test)]
    pub fn new(
        settings_file: String,
        uuid: Option<String>,
        arch: Option<OVMFArch>,
        size: Option<OVMFSize>,
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
            OVMFArch::Arch32 => OVMF32_BOOT_ROM,
            OVMFArch::Arch64 => {
                let size = self.size.unwrap_or_default();
                let secure_boot = self.secure_boot.unwrap_or_default();

                if size == OVMFSize::Size4M {
                    if secure_boot {
                        OVMF64_4M_SECURE_BOOT_ROM
                    } else {
                        OVMF64_4M_BOOT_ROM
                    }
                } else {
                    if secure_boot {
                        OVMF64_2M_SECURE_BOOT_ROM
                    } else {
                        OVMF64_2M_BOOT_ROM
                    }
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
            None => format!(",size={}", OVMF_SIZE.get(&OVMFSize::Size4M).unwrap().1),
            Some(value) => format!(",size={}", OVMF_SIZE.get(value).unwrap().1),
        }
    }
}
impl QemuDevice for OVMF {
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
impl Bios for OVMF {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let actual: OVMF = serde_yaml::from_str(
            r#"
            "#,
        )
        .unwrap();
        let converted = serde_yaml::to_string(&actual).unwrap();
        println!("{}", converted);
        let ovmf = OVMF {
            settings_file: OVMF::settings_file_default(),
            uuid: None,
            arch: None,
            size: None,
            secure_boot: OVMF::secure_boot_default(),
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
        let actual: OVMF = serde_yaml::from_str(
            r#"
                    file: "the_file"
                    uuid: "the_uuid"
                    arch: "64bit"
                    size: "2M"
                    secure_boot: false
                "#,
        )
        .unwrap();
        let ovmf = OVMF {
            settings_file: "the_file".to_string(),
            uuid: Some("the_uuid".to_string()),
            arch: Some(OVMFArch::Arch64),
            size: Some(OVMFSize::Size2M),
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
