use crate::config::storage::{Storage, StorageData};
use crate::config::types::QemuDevice;
use crate::{optional_value_getter, required_value_getter};
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ScsiDrive {
    #[serde(flatten)]
    base: StorageData,
    #[serde(default)]
    discard: Option<String>,
    #[serde(default = "ScsiDrive::cache_default")]
    cache: String,
    #[serde(default = "ScsiDrive::format_default")]
    format: String,
    #[serde(default = "ScsiDrive::detect_zeroes_default")]
    detect_zeroes: String,
    #[serde(default = "ScsiDrive::bus_default")]
    bus: String,
    #[serde(default = "ScsiDrive::rotation_rate_default")]
    rotation_rate: u8,
}

impl ScsiDrive {
    optional_value_getter!(discard("discard"): String);
    required_value_getter!(cache("cache"): String = "none".to_string());
    required_value_getter!(format("format"): String = "raw".to_string());
    required_value_getter!(detect_zeroes("detect-zeroes"): String = "unmap".to_string());
    required_value_getter!(bus("bus"): String = "scsihw0.0".to_string());
    required_value_getter!(rotation_rate("rotation_rate"): u8 = 1);
}

impl QemuDevice for ScsiDrive {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            self.base.drive(vec![format!(
                "id=drive-scsi{}{}{}{}{}",
                index,
                self.discard(),
                self.format(),
                self.cache(),
                self.detect_zeroes()
            )]),
            self.base.device(vec![format!(
                "scsi-hd,scsi-id={},drive=drive-scsi{},id=scsi{}{}{}",
                index,
                index,
                index,
                self.bus(),
                self.rotation_rate()
            )]),
        ]
    }
}

#[typetag::deserialize(name = "scsi-hd")]
impl Storage for ScsiDrive {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_default_values() {
        let storage = ScsiDrive {
            base: StorageData {
                file: "default_file".to_string(),
                boot_index: None,
                extra_drive_options: vec![],
                extra_device_options: vec![],
            },
            discard: None,
            cache: ScsiDrive::cache_default(),
            format: ScsiDrive::format_default(),
            detect_zeroes: ScsiDrive::detect_zeroes_default(),
            bus: ScsiDrive::bus_default(),
            rotation_rate: 1,
        };

        let yaml = r#"
            file: "default_file"
        "#;
        let from_yaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let qemu_args: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-scsi0,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1".to_string(),
        ];
        assert_eq!(storage.get_qemu_args(0), qemu_args);
    }

    #[test]
    fn test_all_valid_values() {
        let storage = ScsiDrive {
            base: StorageData {
                file: "valid_file".to_string(),
                boot_index: Some(1),
                extra_drive_options: vec!["option_1".to_string(), "option_2".to_string()],
                extra_device_options: vec!["option_3".to_string()],
            },
            discard: Some("on".to_string()),
            cache: "write-back".to_string(),
            format: "qcow2".to_string(),
            detect_zeroes: "off".to_string(),
            bus: "scsihw1.2".to_string(),
            rotation_rate: 3,
        };

        let yaml = r#"
            file: "valid_file"
            boot_index: 1
            discard: "on"
            cache: "write-back"
            format: "qcow2"
            detect_zeroes: "off"
            bus: "scsihw1.2"
            rotation_rate: 3

            extra_drive_options:
            - "option_1"
            - "option_2"

            extra_device_options:
            - "option_3"
        "#;
        let from_yaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let expected: Vec<String> = vec![
            "-drive file=valid_file,if=none,aio=io_uring,id=drive-scsi5,discard=on,format=qcow2,cache=write-back,detect-zeroes=off,option_1,option_2".to_string(),
            "-device scsi-hd,scsi-id=5,drive=drive-scsi5,id=scsi5,bus=scsihw1.2,rotation_rate=3,bootindex=1,option_1,option_2".to_string()
        ];

        assert_eq!(storage.get_qemu_args(5), expected);
    }
}
