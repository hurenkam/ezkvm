use crate::config::storage::storage_payload::StoragePayload;
use crate::{optional_value_getter, required_value_getter};
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct VirtioBlkPci {
    #[serde(default)]
    discard: Option<String>,
    #[serde(default = "VirtioBlkPci::cache_default")]
    cache: String,
    #[serde(default = "VirtioBlkPci::format_default")]
    format: String,
    #[serde(default = "VirtioBlkPci::detect_zeroes_default")]
    detect_zeroes: String,
    #[serde(default = "VirtioBlkPci::bus_default")]
    bus: String,
}

impl VirtioBlkPci {
    optional_value_getter!(discard("discard"): String);
    required_value_getter!(cache("cache"): String = "none".to_string());
    required_value_getter!(format("format"): String = "raw".to_string());
    required_value_getter!(detect_zeroes("detect-zeroes"): String = "unmap".to_string());
    required_value_getter!(bus("bus"): String = "pci.0".to_string());
}

#[typetag::deserialize(name = "virtio-blk-pci")]
impl StoragePayload for VirtioBlkPci {
    fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "id=drive-virtio{}{}{}{}{}",
            index,
            self.discard(),
            self.format(),
            self.cache(),
            self.detect_zeroes()
        )]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "virtio-blk-pci,drive=drive-virtio{},id=virtio{}{}",
            index,
            index,
            self.bus(),
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
        let storage = VirtioBlkPci {
            discard: None,
            cache: VirtioBlkPci::cache_default(),
            format: VirtioBlkPci::format_default(),
            detect_zeroes: VirtioBlkPci::detect_zeroes_default(),
            bus: VirtioBlkPci::bus_default(),
        };

        let yaml = r#"
            type: "virtio-blk-pci"
            file: "default_file"
        "#;
        let from_yaml: VirtioBlkPci = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> =
            vec!["id=drive-virtio0,format=raw,cache=none,detect-zeroes=unmap".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["virtio-blk-pci,drive=drive-virtio0,id=virtio0,bus=pci.0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=drive-virtio5,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device virtio-blk-pci,drive=drive-virtio5,id=virtio5,bus=pci.0".to_string()
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    #[test]
    fn test_all_valid_values() {
        let storage = VirtioBlkPci {
            discard: Some("on".to_string()),
            cache: "write-back".to_string(),
            format: "qcow2".to_string(),
            detect_zeroes: "off".to_string(),
            bus: "pci.2".to_string(),
        };

        let yaml = r#"
            type: "virtio-blk-pci"
            file: "valid_file"
            boot_index: 1
            discard: "on"
            cache: "write-back"
            format: "qcow2"
            detect_zeroes: "off"
            bus: "pci.2"

            extra_drive_options:
                - "option_1"
                - "option_2"

            extra_device_options:
                - "option_3"
        "#;
        let from_yaml: VirtioBlkPci = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec![
            "id=drive-virtio5,discard=on,format=qcow2,cache=write-back,detect-zeroes=off"
                .to_string(),
        ];
        assert_eq!(storage.get_drive_options(5), drive_args);

        let device_args: Vec<String> =
            vec!["virtio-blk-pci,drive=drive-virtio5,id=virtio5,bus=pci.2".to_string()];
        assert_eq!(storage.get_device_options(5), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=valid_file,if=none,aio=io_uring,id=drive-virtio5,discard=on,format=qcow2,cache=write-back,detect-zeroes=off,option_1,option_2".to_string(),
            "-device virtio-blk-pci,drive=drive-virtio5,id=virtio5,bus=pci.2,bootindex=1,option_3".to_string()
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }
}
