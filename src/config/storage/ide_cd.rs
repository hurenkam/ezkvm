use crate::config::storage::StoragePayload;
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct IdeCd {
    #[serde(default = "IdeCd::media_default")]
    media: String,
    #[serde(default)]
    unit: u8,
}

impl IdeCd {
    required_value_getter!(media("media"): String = "cdrom".to_string());
}

#[typetag::deserialize(name = "ide-cd")]
impl StoragePayload for IdeCd {
    fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![format!("id=drive-ide{},media={}", index, self.media)]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "ide-cd,bus=ide.{},drive=drive-ide{},id=ide{},unit={}",
            index, index, index, self.unit
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::storage::StorageItem;
    use crate::config::QemuDevice;

    #[test]
    fn test_all_default_values() {
        let storage = IdeCd {
            media: IdeCd::media_default(),
            unit: u8::default(),
        };

        let yaml = r#"
            type: "ide-cd"
            file: "default_file"
        "#;
        let from_yaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec!["id=drive-ide0,media=cdrom".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["ide-cd,bus=ide.0,drive=drive-ide0,id=ide0,unit=0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let qemu_args: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-ide5,media=cdrom".to_string(),
            "-device ide-cd,bus=ide.5,drive=drive-ide5,id=ide5,unit=0".to_string(),
        ];
        assert_eq!(from_yaml.get_qemu_args(5), qemu_args);
    }

    #[test]
    fn test_all_valid_values() {
        let storage = IdeCd {
            media: "dvd".to_string(),
            unit: 3,
        };

        let yaml = r#"
            type: "ide-cd"
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
        let from_yaml: IdeCd = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec!["id=drive-ide5,media=dvd".to_string()];
        assert_eq!(storage.get_drive_options(5), drive_args);

        let device_args: Vec<String> =
            vec!["ide-cd,bus=ide.5,drive=drive-ide5,id=ide5,unit=3".to_string()];
        assert_eq!(storage.get_device_options(5), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let qemu_args: Vec<String> = vec![
            "-drive file=valid_file,if=none,aio=io_uring,id=drive-ide5,media=dvd,option_1,option_2"
                .to_string(),
            "-device ide-cd,bus=ide.5,drive=drive-ide5,id=ide5,unit=3,bootindex=2,option_3"
                .to_string(),
        ];
        assert_eq!(from_yaml.get_qemu_args(5), qemu_args);
    }
}
