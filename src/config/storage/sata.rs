use crate::config::storage::storage_payload::StoragePayload;
use crate::{optional_value_getter, required_value_getter};
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SataDeviceType {
    Cd,
    Hd,
    Ssd,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Sata {
    device_type: SataDeviceType,
    #[serde(default)]
    discard: Option<String>,
    #[serde(default = "Sata::cache_default")]
    cache: String,
    #[serde(default = "Sata::format_default")]
    format: String,
    #[serde(default = "Sata::detect_zeroes_default")]
    detect_zeroes: String,
    #[serde(default = "Sata::bus_default")]
    bus: String,
    #[serde(default = "Sata::rotation_rate_default")]
    rotation_rate: u8,
    #[serde(default = "Sata::unit_default")]
    unit: String,
    #[serde(default = "Sata::media_default")]
    media: String,
}

impl Sata {
    optional_value_getter!(discard("discard"): String);
    required_value_getter!(cache("cache"): String = "none".to_string());
    required_value_getter!(format("format"): String = "raw".to_string());
    required_value_getter!(detect_zeroes("detect-zeroes"): String = "unmap".to_string());
    required_value_getter!(bus("bus"): String = "sata.0".to_string());
    required_value_getter!(rotation_rate("rotation_rate"): u8 = 1);
    required_value_getter!(unit("unit"): String = "0".to_string());
    required_value_getter!(media("media"): String = "cdrom".to_string());

    fn device_type(&self) -> String {
        match self.device_type {
            SataDeviceType::Cd => "cd".to_string(),
            SataDeviceType::Hd => "hd".to_string(),
            SataDeviceType::Ssd => "ssd".to_string(),
        }
    }
    fn id(&self, index: usize) -> String {
        format!(",id=sata{}", index)
    }
    fn drive(&self, index: usize) -> String {
        format!(",drive=drive-sata{}", index)
    }
    fn get_media(&self) -> String {
        match &self.device_type {
            SataDeviceType::Cd => self.media(),
            _ => "".to_string(),
        }
    }
}

#[typetag::deserialize(name = "sata")]
impl StoragePayload for Sata {
    fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "id=drive-sata{}{}{}{}{}{}",
            index,
            self.discard(),
            self.format(),
            self.cache(),
            self.detect_zeroes(),
            self.get_media()
        )]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "sata-{}{}{}{}{}",
            self.device_type(),
            self.bus(),
            self.drive(index),
            self.id(index),
            self.unit(),
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
        let storage = Sata {
            device_type: SataDeviceType::Cd,
            discard: None,
            cache: Sata::cache_default(),
            format: Sata::format_default(),
            detect_zeroes: Sata::detect_zeroes_default(),
            bus: Sata::bus_default(),
            rotation_rate: Sata::rotation_rate_default(),
            unit: Sata::unit_default(),
            media: Sata::media_default(),
        };

        let yaml = r#"
            type: "sata"
            device_type: "cd"
            file: "default_file"
        "#;
        let from_yaml: Sata = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec![
            "id=drive-sata0,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string(),
        ];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["sata-cd,bus=sata.0,drive=drive-sata0,id=sata0,unit=0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-sata5,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string(),
            "-device sata-cd,bus=sata.0,drive=drive-sata5,id=sata5,unit=0".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    #[test]
    fn test_sata_cd() {
        let storage = Sata {
            device_type: SataDeviceType::Cd,
            discard: None,
            cache: Sata::cache_default(),
            format: Sata::format_default(),
            detect_zeroes: Sata::detect_zeroes_default(),
            bus: Sata::bus_default(),
            rotation_rate: Sata::rotation_rate_default(),
            unit: Sata::unit_default(),
            media: "cdrom".to_string(),
        };

        let yaml = r#"
            type: "sata"
            device_type: "cd"
            file: "default_file"
        "#;
        let from_yaml: Sata = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec![
            "id=drive-sata0,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string(),
        ];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["sata-cd,bus=sata.0,drive=drive-sata0,id=sata0,unit=0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-sata5,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string(),
            "-device sata-cd,bus=sata.0,drive=drive-sata5,id=sata5,unit=0".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    #[test]
    fn test_sata_hd() {
        let storage = Sata {
            device_type: SataDeviceType::Hd,
            discard: None,
            cache: Sata::cache_default(),
            format: Sata::format_default(),
            detect_zeroes: Sata::detect_zeroes_default(),
            bus: "sata.0".to_string(),
            rotation_rate: 0,
            unit: "1".to_string(),
            media: "cdrom".to_string(),
        };

        let yaml = r#"
            type: "sata"
            device_type: "hd"
            file: "default_file"
            unit: "1"
            rotation_rate: 0
        "#;
        let from_yaml: Sata = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> =
            vec!["id=drive-sata0,format=raw,cache=none,detect-zeroes=unmap".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["sata-hd,bus=sata.0,drive=drive-sata0,id=sata0,unit=1".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-sata5,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device sata-hd,bus=sata.0,drive=drive-sata5,id=sata5,unit=1".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }
}
