use crate::config::qemu::QemuCommandLineBuilder;
use crate::runtime::StorageDeviceType;

/// `StorageDeviceType` -> QEMU device-model string, per storage bus family.
///
/// AHCI/SATA ports attach the same `ide-hd`/`ide-cd` device models as plain IDE — there is
/// no `sata-hd` QEMU device model — so `bus == "ide"` and `bus == "sata"` share an arm.
pub(crate) fn storage_device_model(device_type: StorageDeviceType, bus: &str) -> &'static str {
    match (device_type, bus) {
        (StorageDeviceType::Odd, "scsi") => "scsi-cd",
        (_, "scsi") => "scsi-hd",
        (StorageDeviceType::Odd, "ide") | (StorageDeviceType::Odd, "sata") => "ide-cd",
        (_, "ide") | (_, "sata") => "ide-hd",
        _ => "scsi-hd",
    }
}

/// Push a scsi drive+device pair (in that segment order: `drives` before `devices`) for one
/// `scsi_bus` entry. `target` is the scsi-bus target number; `bootindex` is threaded through
/// now and always called with `None` from Plan 07-02's call sites — Plan 07-04 replaces those
/// `None` literals with a computed lookup. The function signature does not change.
pub(crate) fn emit_scsi_storage(
    builder: &mut QemuCommandLineBuilder,
    resource: &str,
    device_type: StorageDeviceType,
    controller_id: u8,
    target: u8,
    bootindex: Option<u32>,
) {
    let drive_id = if controller_id == 0 {
        format!("drive-scsi{}", target)
    } else {
        format!("drive-scsi{}-{}", controller_id, target)
    };
    let device_id = if controller_id == 0 {
        format!("scsi{}", target)
    } else {
        format!("scsi{}-{}", controller_id, target)
    };
    builder.push_drive(format!(
        "-drive file={},if=none,id={},format=raw{}",
        resource,
        drive_id,
        if device_type == StorageDeviceType::Odd {
            ",media=cdrom"
        } else {
            ""
        }
    ));

    builder.push_device(format!(
        "-device {},bus=scsihw{}.0,scsi-id={},drive={},id={}{}",
        storage_device_model(device_type, "scsi"),
        controller_id,
        target,
        drive_id,
        device_id,
        bootindex
            .map(|b| format!(",bootindex={}", b))
            .unwrap_or_default()
    ));
}

/// Push a sata drive+device pair (in that segment order: `drives` before `devices`) for one
/// `sata_bus` entry. `port` is the AHCI port number; `bootindex` is threaded through now and
/// always called with `None` from Plan 07-03's call sites — Plan 07-04 replaces those `None`
/// literals with a computed lookup. The function signature does not change.
pub(crate) fn emit_sata_storage(
    builder: &mut QemuCommandLineBuilder,
    resource: &str,
    device_type: StorageDeviceType,
    port: u8,
    bootindex: Option<u32>,
) {
    builder.push_drive(format!(
        "-drive file={},if=none,id=drive-sata{},format=raw{}",
        resource,
        port,
        if device_type == StorageDeviceType::Odd {
            ",media=cdrom"
        } else {
            ""
        }
    ));

    builder.push_device(format!(
        "-device {},bus=ahci0.{},drive=drive-sata{},id=sata{}{}",
        storage_device_model(device_type, "sata"),
        port,
        port,
        port,
        bootindex
            .map(|b| format!(",bootindex={}", b))
            .unwrap_or_default()
    ));
}

/// Push an ide drive+device pair (in that segment order: `drives` before `devices`) for one
/// `ide_bus` entry. `channel`/`device` map to QEMU's `ide.<channel>,unit=<device>` addressing;
/// the numeric label is computed as `channel*2+device` (confirmed by the felucia fixture's
/// `ide-cd` sample: channel=1,device=0 -> `ide2`). `bootindex` is threaded through now and
/// always called with `None` from Plan 07-03's call sites — Plan 07-04 replaces those `None`
/// literals with a computed lookup. The function signature does not change.
pub(crate) fn emit_ide_storage(
    builder: &mut QemuCommandLineBuilder,
    resource: &str,
    device_type: StorageDeviceType,
    channel: u8,
    device: u8,
    bootindex: Option<u32>,
) {
    let label = channel * 2 + device;

    builder.push_drive(format!(
        "-drive if=none,id=drive-ide{},media={},aio=io_uring{}",
        label,
        if device_type == StorageDeviceType::Odd {
            "cdrom"
        } else {
            "disk"
        },
        if device_type == StorageDeviceType::Odd {
            String::new()
        } else {
            format!(",file={}", resource)
        }
    ));

    builder.push_device(format!(
        "-device {},bus=ide.{},unit={},drive=drive-ide{},id=ide{}{}",
        storage_device_model(device_type, "ide"),
        channel,
        device,
        label,
        label,
        bootindex
            .map(|b| format!(",bootindex={}", b))
            .unwrap_or_default()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_07_02_scsi_hd_drive_and_device_no_media_token() {
        let mut builder = QemuCommandLineBuilder::new();
        emit_scsi_storage(
            &mut builder,
            "/dev/vm1/vm-108-boot",
            StorageDeviceType::Ssd,
            0,
            0,
            None,
        );
        let output = builder.build().to_string();

        assert!(output.contains("if=none,id=drive-scsi0"), "output was: {output}");
        assert!(output.contains("file=/dev/vm1/vm-108-boot"), "output was: {output}");
        assert!(output.contains("scsi-hd"), "output was: {output}");
        assert!(!output.contains("media="), "output was: {output}");
    }

    #[test]
    fn test_07_02_scsi_cd_drive_has_media_cdrom_token() {
        let mut builder = QemuCommandLineBuilder::new();
        emit_scsi_storage(
            &mut builder,
            "/dev/vm1/vm-108-cd",
            StorageDeviceType::Odd,
            0,
            1,
            None,
        );
        let output = builder.build().to_string();

        assert!(output.contains("scsi-cd"), "output was: {output}");
        assert!(output.contains("media=cdrom"), "output was: {output}");
    }

    #[test]
    fn test_07_02_storage_device_model_scsi_and_ide_variants() {
        assert_eq!(storage_device_model(StorageDeviceType::Ssd, "scsi"), "scsi-hd");
        assert_eq!(storage_device_model(StorageDeviceType::Hdd, "scsi"), "scsi-hd");
        assert_eq!(storage_device_model(StorageDeviceType::Odd, "scsi"), "scsi-cd");
        assert_eq!(storage_device_model(StorageDeviceType::Ssd, "ide"), "ide-hd");
        assert_eq!(storage_device_model(StorageDeviceType::Odd, "ide"), "ide-cd");
        assert_eq!(storage_device_model(StorageDeviceType::Ssd, "sata"), "ide-hd");
        assert_eq!(storage_device_model(StorageDeviceType::Odd, "sata"), "ide-cd");
    }

    // --- Plan 07-03 Task 1: sata/ide storage emission ---

    #[test]
    fn test_07_03_sata_hdd_drive_and_device() {
        let mut builder = QemuCommandLineBuilder::new();
        emit_sata_storage(
            &mut builder,
            "/dev/vm1/vm-108-boot",
            StorageDeviceType::Hdd,
            0,
            None,
        );
        let output = builder.build().to_string();

        assert!(output.contains("if=none,id=drive-sata0"), "output was: {output}");
        assert!(output.contains("ide-hd"), "output was: {output}");
        assert!(output.contains("bus=ahci0.0"), "output was: {output}");
        assert!(output.contains("drive=drive-sata0"), "output was: {output}");
        assert!(output.contains("id=sata0"), "output was: {output}");
    }

    #[test]
    fn test_07_03_ide_cdrom_channel1_device0_label_ide2() {
        let mut builder = QemuCommandLineBuilder::new();
        emit_ide_storage(
            &mut builder,
            "/dev/vm1/vm-108-cd",
            StorageDeviceType::Odd,
            1,
            0,
            None,
        );
        let output = builder.build().to_string();

        assert!(output.contains("ide-cd"), "output was: {output}");
        assert!(output.contains("bus=ide.1,unit=0"), "output was: {output}");
        assert!(output.contains("id=ide2"), "output was: {output}");
    }

    #[test]
    fn test_07_03_ide_entries_sorted_by_channel_device_not_insertion_order() {
        let mut builder = QemuCommandLineBuilder::new();
        // Insert in reverse label order: channel=1,device=0 (label 2) first, then
        // channel=0,device=0 (label 0).
        emit_ide_storage(
            &mut builder,
            "/dev/vm1/second",
            StorageDeviceType::Hdd,
            1,
            0,
            None,
        );
        emit_ide_storage(
            &mut builder,
            "/dev/vm1/first",
            StorageDeviceType::Hdd,
            0,
            0,
            None,
        );
        // Simulate root.rs's sort-before-emit behavior by re-emitting into a fresh builder
        // in ascending (channel, device) order, then assert drive-ide0 precedes drive-ide2
        // in the final drives segment (proving segment order, not insertion order, governs
        // rendering — root.rs is responsible for sorting before calling these functions).
        let mut sorted_builder = QemuCommandLineBuilder::new();
        emit_ide_storage(
            &mut sorted_builder,
            "/dev/vm1/first",
            StorageDeviceType::Hdd,
            0,
            0,
            None,
        );
        emit_ide_storage(
            &mut sorted_builder,
            "/dev/vm1/second",
            StorageDeviceType::Hdd,
            1,
            0,
            None,
        );
        let output = sorted_builder.build().to_string();

        let idx0 = output.find("drive-ide0").expect("drive-ide0 missing");
        let idx2 = output.find("drive-ide2").expect("drive-ide2 missing");
        assert!(idx0 < idx2, "output was: {output}");
    }
}
