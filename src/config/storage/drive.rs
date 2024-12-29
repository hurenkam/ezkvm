use crate::{optional_value_getter2, required_value_getter2};
use paste::paste;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Drive {
    #[serde(rename = "type")]
    drive_type: String,
    file: String,
    #[serde(default)]
    discard: Option<String>,
    #[serde(default = "Drive::cache_default")]
    cache: String,
    #[serde(default = "Drive::format_default")]
    format: String,
    #[serde(default = "Drive::detect_zeroes_default")]
    detect_zeroes: String,
    #[serde(default)]
    rotation_rate: Option<u8>,
    #[serde(default)]
    boot_index: Option<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_drive_options: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_device_options: Vec<String>,
}
impl Drive {
    optional_value_getter2!(discard("discard"): String);
    required_value_getter2!(cache("cache"): String = "none".to_string());
    required_value_getter2!(format("format"): String = "raw".to_string());
    required_value_getter2!(detect_zeroes("detect-zeroes"): String = "unmap".to_string());
    optional_value_getter2!(rotation_rate("rotation_rate"): u8);
    optional_value_getter2!(boot_index("boot_index"): u8);

    #[allow(unused)]
    pub fn new(drive_type: String, file: String) -> Self {
        Self {
            drive_type,
            file,
            discard: None,
            cache: Self::cache_default(),
            format: Self::format_default(),
            detect_zeroes: Self::detect_zeroes_default(),
            rotation_rate: None,
            boot_index: None,
            extra_drive_options: vec![],
            extra_device_options: vec![],
        }
    }
    pub fn get_drive_type(&self) -> &str {
        &self.drive_type
    }
    pub(crate) fn get_drive_options(&self) -> Vec<String> {
        let mut result = vec![format!("file={}", self.file), "if=none".to_string()];
        if let Some(value) = self.discard() {
            result.push(value);
        }
        if let Some(value) = self.format() {
            result.push(value);
        }
        if let Some(value) = self.cache() {
            result.push(value);
        }
        if let Some(value) = self.detect_zeroes() {
            result.push(value);
        }
        result.extend(self.extra_drive_options.clone());
        result
    }
    pub(crate) fn get_device_options(&self) -> Vec<String> {
        let mut result = vec![];
        if let Some(value) = self.rotation_rate() {
            result.push(value);
        }
        if let Some(value) = self.boot_index() {
            result.push(value);
        }
        result.extend(self.extra_device_options.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_all_default_values() {
        let storage = Drive {
            drive_type: "".to_string(),
            file: "".to_string(),
            discard: None,
            cache: Drive::cache_default(),
            format: Drive::format_default(),
            detect_zeroes: Drive::detect_zeroes_default(),
            rotation_rate: None,
            boot_index: None,
            extra_drive_options: vec![],
            extra_device_options: vec![],
        };

        let yaml = r#"
            type: ""
            file: ""
        "#;
        let from_yaml: Drive = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec![
            "file=".to_string(),
            "if=none".to_string(),
            "format=raw".to_string(),
            "cache=none".to_string(),
            "detect-zeroes=unmap".to_string(),
        ];
        assert_eq!(storage.get_drive_options(), drive_args);

        let device_args: Vec<String> = vec![];
        assert_eq!(storage.get_device_options(), device_args);
    }

    #[test]
    fn test_all_valid_values() {
        let storage = Drive {
            drive_type: "cd".to_string(),
            file: "file.img".to_string(),
            discard: Some("on".to_string()),
            cache: "write-back".to_string(),
            format: "qcow2".to_string(),
            detect_zeroes: "off".to_string(),
            rotation_rate: Some(3),
            boot_index: Some(1),
            extra_drive_options: vec!["option_1".to_string(), "option_2".to_string()],
            extra_device_options: vec!["option_3".to_string()],
        };

        let yaml = r#"
            type: "cd"
            file: "file.img"
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
        let from_yaml: Drive = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec![
            "file=file.img".to_string(),
            "if=none".to_string(),
            "discard=on".to_string(),
            "format=qcow2".to_string(),
            "cache=write-back".to_string(),
            "detect-zeroes=off".to_string(),
            "option_1".to_string(),
            "option_2".to_string(),
        ];
        assert_eq!(storage.get_drive_options(), drive_args);

        let device_args: Vec<String> = vec![
            "rotation_rate=3".to_string(),
            "boot_index=1".to_string(),
            "option_3".to_string(),
        ];
        assert_eq!(storage.get_device_options(), device_args);
    }
}
