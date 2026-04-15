use super::*;
use crate::qemu::types::QemuArgs;

#[test]
fn drive_serialization_omits_default_false_flags() {
    let drive = DriveConfig {
        id: "ide2".to_string(),
        path: "".to_string(),
        interface: "ide".to_string(),
        r#type: "cdrom".to_string(),
        format: "raw".to_string(),
        readonly: false,
        discard: false,
        ssd: false,
        cache: None,
        aio: None,
        detect_zeroes: None,
        controller: None,
        boot_index: Some(101),
        scsi_id: None,
        rotation_rate: None,
        bus: None,
        unit: None,
    };

    let yaml = serde_yaml::to_string(&drive).expect("serialize drive");
    assert!(!yaml.contains("readonly:"));
    assert!(!yaml.contains("discard:"));
    assert!(!yaml.contains("ssd:"));
}

#[test]
fn drive_roundtrip_preserves_qemu_args() {
    let drive = DriveConfig {
        id: "scsi0".to_string(),
        path: "/dev/vm1/vm-108-boot".to_string(),
        interface: "scsi".to_string(),
        r#type: "disk".to_string(),
        format: "raw".to_string(),
        readonly: false,
        discard: true,
        ssd: true,
        cache: Some("none".to_string()),
        aio: Some("io_uring".to_string()),
        detect_zeroes: Some("unmap".to_string()),
        controller: Some("scsihw0".to_string()),
        boot_index: Some(100),
        scsi_id: None,
        rotation_rate: Some(1),
        bus: None,
        unit: None,
    };

    let yaml = serde_yaml::to_string(&drive).expect("serialize drive");
    let parsed: DriveConfig = serde_yaml::from_str(&yaml).expect("deserialize drive");

    let expected_args = QemuArgs::from(drive).into_inner();
    let actual_args = QemuArgs::from(parsed).into_inner();
    assert_eq!(actual_args, expected_args);
}
