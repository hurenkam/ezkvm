use crate::config::storage::{Storage, StorageData};
use crate::config::types::QemuDevice;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ScsiDrive {
    #[serde(flatten)]
    base: StorageData,
    #[serde(default)]
    discard: Option<bool>,
    #[serde(default = "default_cache")]
    cache: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default = "default_detect_zeroes")]
    detect_zeroes: String,
    #[serde(default = "default_bus")]
    bus: String,
    #[serde(default = "default_rotation_rate")]
    rotation_rate: u8,
}

fn default_cache() -> String {
    "none".to_string()
}
fn default_format() -> String {
    "raw".to_string()
}
fn default_detect_zeroes() -> String {
    "unmap".to_string()
}
fn default_bus() -> String {
    "scsihw0.0".to_string()
}
fn default_rotation_rate() -> u8 {
    1
}

impl QemuDevice for ScsiDrive {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let discard = match self.discard {
            None => "",
            Some(discard) => {
                if discard {
                    ",discard=on"
                } else {
                    ",discard=off"
                }
            }
        };

        vec![
            self.base.drive(vec![format!(
                "id=drive-scsi{}{},format={},cache={},detect-zeroes={}",
                index, discard, self.format, self.cache, self.detect_zeroes
            )]),
            self.base.device(vec![format!(
                "scsi-hd,scsi-id={},drive=drive-scsi{},id=scsi{},bus={},rotation_rate={}",
                index, index, index, self.bus, self.rotation_rate
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
            cache: default_cache(),
            format: default_format(),
            detect_zeroes: default_detect_zeroes(),
            bus: default_bus(),
            rotation_rate: default_rotation_rate(),
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
            discard: Some(true),
            cache: "write-back".to_string(),
            format: "qcow2".to_string(),
            detect_zeroes: "off".to_string(),
            bus: "scsihw1.2".to_string(),
            rotation_rate: 3,
        };

        let yaml = r#"
            file: "valid_file"
            boot_index: 1
            discard: true
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
