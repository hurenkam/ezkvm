use crate::{optional_value_getter2, required_value_getter2};
use paste::paste;
use serde::Deserialize;

/// Per-drive throttle bandwidth/IOPS limits.
///
/// YAML example:
/// ```yaml
/// throttle:
///   bps_read: 104857600   # 100 MiB/s
///   iops_write: 500
/// ```
#[derive(Deserialize, Debug, PartialEq, Clone, Default)]
pub struct ThrottleConfig {
    #[serde(default)]
    bps_total: Option<u64>,
    #[serde(default)]
    bps_read: Option<u64>,
    #[serde(default)]
    bps_write: Option<u64>,
    #[serde(default)]
    iops_total: Option<u64>,
    #[serde(default)]
    iops_read: Option<u64>,
    #[serde(default)]
    iops_write: Option<u64>,
}

impl ThrottleConfig {
    pub fn get_drive_options(&self) -> Vec<String> {
        let mut result = vec![];
        if let Some(v) = self.bps_total {
            result.push(format!("throttling.bps-total={}", v));
        }
        if let Some(v) = self.bps_read {
            result.push(format!("throttling.bps-read={}", v));
        }
        if let Some(v) = self.bps_write {
            result.push(format!("throttling.bps-write={}", v));
        }
        if let Some(v) = self.iops_total {
            result.push(format!("throttling.iops-total={}", v));
        }
        if let Some(v) = self.iops_read {
            result.push(format!("throttling.iops-read={}", v));
        }
        if let Some(v) = self.iops_write {
            result.push(format!("throttling.iops-write={}", v));
        }
        result
    }
}

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
    /// AIO mode: "threads" (default), "native", or "io_uring".
    #[serde(default)]
    aio: Option<String>,
    /// Drive serial number reported to the guest.
    #[serde(default)]
    serial: Option<String>,
    /// Enable snapshot mode (writes go to a temporary overlay).
    #[serde(default)]
    snapshot: Option<bool>,
    /// Write error policy: "enospc", "stop", "report", "ignore".
    #[serde(default)]
    werror: Option<String>,
    /// Read error policy: "report" (default), "ignore", "stop".
    #[serde(default)]
    rerror: Option<String>,
    /// Iothread ID to use for this drive (device-side option).
    /// The matching `-object iothread,id=<value>` must be added via `extras`.
    #[serde(default)]
    iothread: Option<String>,
    /// Per-drive bandwidth and IOPS throttle settings.
    #[serde(default)]
    throttle: Option<ThrottleConfig>,
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
            aio: None,
            serial: None,
            snapshot: None,
            werror: None,
            rerror: None,
            iothread: None,
            throttle: None,
            extra_drive_options: vec![],
            extra_device_options: vec![],
        }
    }
    pub fn get_drive_type(&self) -> &str {
        &self.drive_type
    }

    fn is_empty_cd_tray(&self) -> bool {
        self.drive_type.eq_ignore_ascii_case("cd")
            && (self.file.trim().is_empty() || self.file.eq_ignore_ascii_case("none"))
    }

    pub(crate) fn get_drive_options(&self) -> Vec<String> {
        if self.is_empty_cd_tray() {
            let mut result = vec!["if=none".to_string(), "media=cdrom".to_string()];
            result.extend(self.extra_drive_options.clone());
            return result;
        }

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
        if let Some(ref aio) = self.aio {
            result.push(format!("aio={}", aio));
        }
        if self.snapshot == Some(true) {
            result.push("snapshot=on".to_string());
        }
        if let Some(ref policy) = self.werror {
            result.push(format!("werror={}", policy));
        }
        if let Some(ref policy) = self.rerror {
            result.push(format!("rerror={}", policy));
        }
        if let Some(ref throttle) = self.throttle {
            result.extend(throttle.get_drive_options());
        }
        result.extend(self.extra_drive_options.clone());
        result
    }

    pub(crate) fn get_device_options(&self) -> Vec<String> {
        let mut result = vec![];
        if let Some(ref serial) = self.serial {
            result.push(format!("serial={}", serial));
        }
        if let Some(value) = self.rotation_rate() {
            result.push(value);
        }
        if let Some(value) = self.boot_index() {
            result.push(value);
        }
        if let Some(ref id) = self.iothread {
            result.push(format!("iothread={}", id));
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
            aio: None,
            serial: None,
            snapshot: None,
            werror: None,
            rerror: None,
            iothread: None,
            throttle: None,
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
            aio: None,
            serial: None,
            snapshot: None,
            werror: None,
            rerror: None,
            iothread: None,
            throttle: None,
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

    #[test]
    fn test_empty_cd_tray_is_rendered_without_backing_file() {
        let storage = Drive::new("cd".to_string(), "none".to_string());

        let drive_args: Vec<String> = vec!["if=none".to_string(), "media=cdrom".to_string()];
        assert_eq!(storage.get_drive_options(), drive_args);
    }

    #[test]
    fn test_aio_io_uring() {
        let storage = Drive {
            drive_type: "hd".to_string(),
            file: "disk.qcow2".to_string(),
            discard: None,
            cache: "none".to_string(),
            format: "qcow2".to_string(),
            detect_zeroes: "unmap".to_string(),
            rotation_rate: None,
            boot_index: None,
            aio: Some("io_uring".to_string()),
            serial: None,
            snapshot: None,
            werror: None,
            rerror: None,
            iothread: None,
            throttle: None,
            extra_drive_options: vec![],
            extra_device_options: vec![],
        };
        let opts = storage.get_drive_options();
        assert!(opts.contains(&"aio=io_uring".to_string()));
    }

    #[test]
    fn test_snapshot_mode() {
        let storage = Drive {
            drive_type: "hd".to_string(),
            file: "disk.img".to_string(),
            discard: None,
            cache: Drive::cache_default(),
            format: Drive::format_default(),
            detect_zeroes: Drive::detect_zeroes_default(),
            rotation_rate: None,
            boot_index: None,
            aio: None,
            serial: None,
            snapshot: Some(true),
            werror: None,
            rerror: None,
            iothread: None,
            throttle: None,
            extra_drive_options: vec![],
            extra_device_options: vec![],
        };
        assert!(storage.get_drive_options().contains(&"snapshot=on".to_string()));
    }

    #[test]
    fn test_error_policy() {
        let storage = Drive {
            drive_type: "hd".to_string(),
            file: "disk.img".to_string(),
            discard: None,
            cache: Drive::cache_default(),
            format: Drive::format_default(),
            detect_zeroes: Drive::detect_zeroes_default(),
            rotation_rate: None,
            boot_index: None,
            aio: None,
            serial: None,
            snapshot: None,
            werror: Some("enospc".to_string()),
            rerror: Some("ignore".to_string()),
            iothread: None,
            throttle: None,
            extra_drive_options: vec![],
            extra_device_options: vec![],
        };
        let opts = storage.get_drive_options();
        assert!(opts.contains(&"werror=enospc".to_string()));
        assert!(opts.contains(&"rerror=ignore".to_string()));
    }

    #[test]
    fn test_serial_and_iothread_in_device_options() {
        let storage = Drive {
            drive_type: "hd".to_string(),
            file: "disk.img".to_string(),
            discard: None,
            cache: Drive::cache_default(),
            format: Drive::format_default(),
            detect_zeroes: Drive::detect_zeroes_default(),
            rotation_rate: None,
            boot_index: None,
            aio: None,
            serial: Some("DRIVESN001".to_string()),
            snapshot: None,
            werror: None,
            rerror: None,
            iothread: Some("iothread0".to_string()),
            throttle: None,
            extra_drive_options: vec![],
            extra_device_options: vec![],
        };
        let dev_opts = storage.get_device_options();
        assert!(dev_opts.contains(&"serial=DRIVESN001".to_string()));
        assert!(dev_opts.contains(&"iothread=iothread0".to_string()));
    }

    #[test]
    fn test_throttle_config() {
        let storage = Drive {
            drive_type: "hd".to_string(),
            file: "disk.img".to_string(),
            discard: None,
            cache: Drive::cache_default(),
            format: Drive::format_default(),
            detect_zeroes: Drive::detect_zeroes_default(),
            rotation_rate: None,
            boot_index: None,
            aio: None,
            serial: None,
            snapshot: None,
            werror: None,
            rerror: None,
            iothread: None,
            throttle: Some(ThrottleConfig {
                bps_read: Some(104857600),
                iops_write: Some(500),
                ..ThrottleConfig::default()
            }),
            extra_drive_options: vec![],
            extra_device_options: vec![],
        };
        let opts = storage.get_drive_options();
        assert!(opts.contains(&"throttling.bps-read=104857600".to_string()));
        assert!(opts.contains(&"throttling.iops-write=500".to_string()));
    }

    #[test]
    fn test_new_fields_from_yaml() {
        let yaml = r#"
            type: hd
            file: /dev/vg0/vm-disk
            aio: io_uring
            serial: SN-1234
            snapshot: false
            werror: enospc
            rerror: report
            iothread: iothread0
            throttle:
              bps_total: 209715200
              iops_read: 1000
        "#;
        let drive: Drive = serde_yaml::from_str(yaml).unwrap();
        let drive_opts = drive.get_drive_options();
        let dev_opts = drive.get_device_options();
        assert!(drive_opts.contains(&"aio=io_uring".to_string()));
        assert!(drive_opts.contains(&"werror=enospc".to_string()));
        assert!(drive_opts.contains(&"rerror=report".to_string()));
        assert!(drive_opts.contains(&"throttling.bps-total=209715200".to_string()));
        assert!(drive_opts.contains(&"throttling.iops-read=1000".to_string()));
        assert!(dev_opts.contains(&"serial=SN-1234".to_string()));
        assert!(dev_opts.contains(&"iothread=iothread0".to_string()));
        // snapshot: false should NOT emit snapshot=on
        assert!(!drive_opts.contains(&"snapshot=on".to_string()));
    }
}
