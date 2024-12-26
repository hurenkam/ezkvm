use crate::config::storage::storage_payload::StoragePayload;
use crate::{optional_value_getter, required_value_getter};
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Default, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum IdeDeviceType {
    #[default]
    Cd,
    Hd,
    Ssd,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Ide {
    device_type: IdeDeviceType,
    #[serde(default)]
    discard: Option<String>,
    #[serde(default = "Ide::cache_default")]
    cache: String,
    #[serde(default = "Ide::format_default")]
    format: String,
    #[serde(default = "Ide::detect_zeroes_default")]
    detect_zeroes: String,
    #[serde(default = "Ide::bus_default")]
    bus: String,
    #[serde(default = "Ide::rotation_rate_default")]
    rotation_rate: u8,
    #[serde(default = "Ide::unit_default")]
    unit: String,
    #[serde(default = "Ide::media_default")]
    media: String,
}

impl Ide {
    optional_value_getter!(discard("discard"): String);
    required_value_getter!(cache("cache"): String = "none".to_string());
    required_value_getter!(format("format"): String = "raw".to_string());
    required_value_getter!(detect_zeroes("detect-zeroes"): String = "unmap".to_string());
    required_value_getter!(bus("bus"): String = "ide.0".to_string());
    required_value_getter!(rotation_rate("rotation_rate"): u8 = 1);
    required_value_getter!(unit("unit"): String = "0".to_string());
    required_value_getter!(media("media"): String = "cdrom".to_string());

    fn device_type(&self) -> String {
        match self.device_type {
            IdeDeviceType::Cd { .. } => "cd".to_string(),
            IdeDeviceType::Hd => "hd".to_string(),
            IdeDeviceType::Ssd => "ssd".to_string(),
        }
    }
    fn id(&self, index: usize) -> String {
        format!(",id=ide{}", index)
    }
    fn drive(&self, index: usize) -> String {
        format!(",drive=drive-ide{}", index)
    }
    fn get_media(&self) -> String {
        match &self.device_type {
            IdeDeviceType::Cd => self.media(),
            _ => "".to_string(),
        }
    }
}

#[typetag::deserialize(name = "ide")]
impl StoragePayload for Ide {
    fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "id=drive-ide{}{}{}{}{}{}",
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
            "ide-{}{}{}{}{}",
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
        let storage = Ide {
            device_type: IdeDeviceType::default(),
            discard: None,
            cache: Ide::cache_default(),
            format: Ide::format_default(),
            detect_zeroes: Ide::detect_zeroes_default(),
            bus: Ide::bus_default(),
            rotation_rate: Ide::rotation_rate_default(),
            unit: Ide::unit_default(),
            media: Ide::media_default(),
        };

        let yaml = r#"
            type: "ide"
            device_type: "cd"
            file: "default_file"
        "#;
        let from_yaml: Ide = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> =
            vec!["id=drive-ide0,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["ide-cd,bus=ide.0,drive=drive-ide0,id=ide0,unit=0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-ide5,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string(),
            "-device ide-cd,bus=ide.0,drive=drive-ide5,id=ide5,unit=0".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    #[test]
    fn test_ide_cd() {
        let storage = Ide {
            device_type: IdeDeviceType::Cd,
            discard: None,
            cache: Ide::cache_default(),
            format: Ide::format_default(),
            detect_zeroes: Ide::detect_zeroes_default(),
            bus: "ide.0".to_string(),
            rotation_rate: Ide::rotation_rate_default(),
            unit: "0".to_string(),
            media: "cdrom".to_string(),
        };
        let converted = serde_yaml::to_string(&storage).unwrap();
        println!("{}", converted);

        let yaml = r#"
            type: "ide"
            device_type: "cd"
            file: "default_file"
        "#;
        let from_yaml: Ide = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> =
            vec!["id=drive-ide0,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["ide-cd,bus=ide.0,drive=drive-ide0,id=ide0,unit=0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-ide5,format=raw,cache=none,detect-zeroes=unmap,media=cdrom".to_string(),
            "-device ide-cd,bus=ide.0,drive=drive-ide5,id=ide5,unit=0".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    #[test]
    fn test_ide_hd() {
        let storage = Ide {
            device_type: IdeDeviceType::Hd,
            discard: None,
            cache: Ide::cache_default(),
            format: Ide::format_default(),
            detect_zeroes: Ide::detect_zeroes_default(),
            bus: "ide.0".to_string(),
            rotation_rate: Ide::rotation_rate_default(),
            unit: "1".to_string(),
            media: Ide::media_default(),
        };

        let yaml = r#"
            type: "ide"
            device_type: "hd"
            file: "default_file"
            unit: "1"
        "#;
        let from_yaml: Ide = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> =
            vec!["id=drive-ide0,format=raw,cache=none,detect-zeroes=unmap".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["ide-hd,bus=ide.0,drive=drive-ide0,id=ide0,unit=1".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-ide5,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device ide-hd,bus=ide.0,drive=drive-ide5,id=ide5,unit=1".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }
}
