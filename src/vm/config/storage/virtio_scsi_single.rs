use super::drive::Drive;
use super::Controller;
use super::QemuDevice;
use serde::Deserialize;

/// A SCSI controller that uses the `virtio-scsi-single` topology: one
/// `virtio-scsi-pci` device and one `iothread` per drive.  This matches the
/// layout Proxmox emits when `scsihw: virtio-scsi-single` is set.
///
/// YAML example:
/// ```yaml
/// storage:
/// - controller: virtio-scsi-single
///   drives:
///   - type: hd
///     file: /dev/vg/disk0
///     discard: on
///   - type: hd
///     file: /dev/vg/disk1
///     cache: writeback
/// ```
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct VirtioScsiSingleController {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    drives: Vec<Drive>,
}

impl QemuDevice for VirtioScsiSingleController {
    fn get_qemu_args(&self, controller_index: usize) -> Vec<String> {
        let mut result = vec![];
        for (drive_index, drive) in self.drives.iter().enumerate() {
            let global_index = controller_index + drive_index + self.offset;
            let iothread_id = format!("iothread-virtioscsi{}", global_index);
            let ctrl_id = format!("virtioscsi{}", global_index);
            let drive_id = format!("drive-virtioscsi{}", global_index);

            // iothread object
            result.push(format!("-object iothread,id={}", iothread_id));

            // virtio-scsi-pci controller device
            result.push(format!(
                "-device virtio-scsi-pci,id={},iothread={}",
                ctrl_id, iothread_id
            ));

            // drive block backend
            let mut drive_args: Vec<String> = vec![format!("id={}", drive_id)];
            drive_args.extend(drive.get_drive_options());
            result.push(format!("-drive {}", drive_args.join(",")));

            // scsi device
            let mut device_args: Vec<String> = vec![
                format!("scsi-{}", drive.get_drive_type()),
                format!("drive={}", drive_id),
                format!("bus={}.0", ctrl_id),
            ];
            device_args.extend(drive.get_device_options());
            result.push(format!("-device {}", device_args.join(",")));
        }
        result
    }
}

#[typetag::deserialize(name = "virtio-scsi-single")]
impl Controller for VirtioScsiSingleController {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_controller() {
        let ctrl = VirtioScsiSingleController {
            offset: 0,
            drives: vec![],
        };
        assert_eq!(ctrl.get_qemu_args(0), Vec::<String>::new());
    }

    #[test]
    fn test_single_drive() {
        let ctrl = VirtioScsiSingleController {
            offset: 0,
            drives: vec![Drive::new("hd".to_string(), "/dev/vg/disk0".to_string())],
        };

        let args = ctrl.get_qemu_args(0);
        assert_eq!(args[0], "-object iothread,id=iothread-virtioscsi0");
        assert_eq!(
            args[1],
            "-device virtio-scsi-pci,id=virtioscsi0,iothread=iothread-virtioscsi0"
        );
        assert!(args[2].starts_with("-drive id=drive-virtioscsi0,"));
        assert!(args[3].starts_with("-device scsi-hd,drive=drive-virtioscsi0,bus=virtioscsi0.0"));
    }

    #[test]
    fn test_two_drives_get_separate_controllers() {
        let ctrl = VirtioScsiSingleController {
            offset: 0,
            drives: vec![
                Drive::new("hd".to_string(), "/dev/vg/disk0".to_string()),
                Drive::new("hd".to_string(), "/dev/vg/disk1".to_string()),
            ],
        };

        let args = ctrl.get_qemu_args(0);
        // First drive: indices 0
        assert!(args[0].contains("iothread-virtioscsi0"));
        assert!(args[1].contains("virtioscsi0"));
        // Second drive: indices 1
        assert!(args[4].contains("iothread-virtioscsi1"));
        assert!(args[5].contains("virtioscsi1"));
        assert_eq!(args.len(), 8); // 4 args per drive × 2 drives
    }

    #[test]
    fn test_controller_index_offsets_global_ids() {
        let ctrl = VirtioScsiSingleController {
            offset: 0,
            drives: vec![Drive::new("hd".to_string(), "/dev/vg/disk0".to_string())],
        };

        // When positioned as the second storage controller (index=2)
        let args = ctrl.get_qemu_args(2);
        assert!(args[0].contains("iothread-virtioscsi2"));
        assert!(args[1].contains("virtioscsi2"));
        assert!(args[2].contains("drive-virtioscsi2"));
        assert!(args[3].contains("virtioscsi2.0"));
    }

    #[test]
    fn test_yaml_deserialization() {
        let yaml = r#"
            drives:
            - type: hd
              file: /dev/vg/disk0
            - type: hd
              file: /dev/vg/disk1
        "#;
        let ctrl: VirtioScsiSingleController = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ctrl.drives.len(), 2);
        // Emits 4 args per drive
        assert_eq!(ctrl.get_qemu_args(0).len(), 8);
    }
}
