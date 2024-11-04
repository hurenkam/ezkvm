use crate::config::storage::StoragePayload;
use crate::{optional_value_getter, required_value_getter};
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ScsiDrive {
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

#[typetag::deserialize(name = "scsi-hd")]
impl StoragePayload for ScsiDrive {
    fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "id=drive-scsi{}{}{}{}{}",
            index,
            self.discard(),
            self.format(),
            self.cache(),
            self.detect_zeroes()
        )]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "scsi-hd,scsi-id={},drive=drive-scsi{},id=scsi{}{}{}",
            index,
            index,
            index,
            self.bus(),
            self.rotation_rate()
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
        let storage = ScsiDrive {
            discard: None,
            cache: ScsiDrive::cache_default(),
            format: ScsiDrive::format_default(),
            detect_zeroes: ScsiDrive::detect_zeroes_default(),
            bus: ScsiDrive::bus_default(),
            rotation_rate: 1,
        };

        let yaml = r#"
            type: "scsi-hd"
            file: "default_file"
        "#;
        let from_yaml: ScsiDrive = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> =
            vec!["id=drive-scsi0,format=raw,cache=none,detect-zeroes=unmap".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> = vec![
            "scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1"
                .to_string(),
        ];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-scsi5,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device scsi-hd,scsi-id=5,drive=drive-scsi5,id=scsi5,bus=scsihw0.0,rotation_rate=1".to_string()
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    #[test]
    fn test_all_valid_values() {
        let storage = ScsiDrive {
            discard: Some("on".to_string()),
            cache: "write-back".to_string(),
            format: "qcow2".to_string(),
            detect_zeroes: "off".to_string(),
            bus: "scsihw1.2".to_string(),
            rotation_rate: 3,
        };

        let yaml = r#"
            type: "scsi-hd"
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

        let drive_args: Vec<String> = vec![
            "id=drive-scsi5,discard=on,format=qcow2,cache=write-back,detect-zeroes=off".to_string(),
        ];
        assert_eq!(storage.get_drive_options(5), drive_args);

        let device_args: Vec<String> = vec![
            "scsi-hd,scsi-id=5,drive=drive-scsi5,id=scsi5,bus=scsihw1.2,rotation_rate=3"
                .to_string(),
        ];
        assert_eq!(storage.get_device_options(5), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
           "-drive file=valid_file,if=none,aio=io_uring,id=drive-scsi5,discard=on,format=qcow2,cache=write-back,detect-zeroes=off,option_1,option_2".to_string(),
           "-device scsi-hd,scsi-id=5,drive=drive-scsi5,id=scsi5,bus=scsihw1.2,rotation_rate=3,bootindex=1,option_3".to_string()
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }
}
