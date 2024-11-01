use crate::config::storage::{Storage, StorageData};
use crate::config::QemuDevice;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct IdeCd {
    #[serde(flatten)]
    base: StorageData,
    #[serde(default = "default_media")]
    media: String,
    #[serde(default)]
    unit: u8,
}

#[typetag::deserialize(name = "ide-cd")]
impl Storage for IdeCd {}

pub fn default_media() -> String {
    "cdrom".to_string()
}
impl QemuDevice for IdeCd {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            self.base
                .drive(vec![format!("id=drive-ide{},media={}", index, self.media)]),
            self.base.device(vec![format!(
                "ide-cd,bus=ide.{},drive=drive-ide{},id=ide{},unit={}",
                index, index, index, self.unit
            )]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_default_values() {
        let storage = IdeCd {
            base: StorageData {
                file: "default_file".to_string(),
                boot_index: None,
                extra_drive_options: vec![],
                extra_device_options: vec![],
            },
            media: default_media(),
            unit: u8::default(),
        };

        let yaml = r#"
            file: "default_file"
        "#;
        let from_yaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let qemu_args: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-ide0,media=cdrom".to_string(),
            "-device ide-cd,bus=ide.0,drive=drive-ide0,id=ide0,unit=0".to_string(),
        ];
        assert_eq!(storage.get_qemu_args(0), qemu_args);
    }

    #[test]
    fn test_all_valid_values() {
        let storage = IdeCd {
            base: StorageData {
                file: "valid_file".to_string(),
                boot_index: Some(2),
                extra_drive_options: vec!["option_1".to_string(), "option_2".to_string()],
                extra_device_options: vec!["option_3".to_string()],
            },
            media: "dvd".to_string(),
            unit: 3,
        };

        let yaml = r#"
            file: "valid_file"
            boot_index: 2
            media: "dvd"
            unit: 3

            extra_drive_options:
            - "option_1"
            - "option_2"

            extra_device_options:
            - "option_3"
        "#;
        let from_yaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let qemu_args: Vec<String> = vec![
            "-drive file=valid_file,if=none,aio=io_uring,id=drive-ide5,media=dvd,option_1,option_2".to_string(),
            "-device ide-cd,bus=ide.5,drive=drive-ide5,id=ide5,unit=3,bootindex=2,option_1,option_2".to_string()
        ];
        assert_eq!(storage.get_qemu_args(5), qemu_args);
    }
}
