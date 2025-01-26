use super::drive::Drive;
use super::Controller;
use super::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct SataController {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    drives: Vec<Drive>,
}

impl QemuDevice for SataController {
    fn get_qemu_args(&self, controller_index: usize) -> Vec<String> {
        let id = format!("ahci{}", controller_index);
        let bus = "pci.0".to_string();
        let address = self.offset + controller_index;
        let mut result = vec![format!(
            "-device ahci,id={},multifunction=on,bus={},addr={}",
            id.clone(),
            bus,
            address
        )];
        for (index, drive) in self.drives.iter().enumerate() {
            let mut drive_args: Vec<String> = vec![format!("id=drive-sata{}", index)];
            drive_args.extend(drive.get_drive_options());

            let mut device_args: Vec<String> = vec![
                format!("ide-{}", drive.get_drive_type()),
                format!("id=sata{}", index),
                format!("drive=drive-sata{}", index),
                format!("bus=ahci{}.{}", controller_index, index),
            ];
            device_args.extend(drive.get_device_options());

            result.extend(vec![
                format!("-drive {}", drive_args.join(",")),
                format!("-device {}", device_args.join(",")),
            ]);
        }
        result
    }
}

#[typetag::deserialize(name = "sata")]
impl Controller for SataController {}

#[cfg(test)]
mod tests {
    use super::QemuDevice;
    use super::*;

    #[test]
    fn test_all_default_values() {
        let storage = SataController {
            offset: 0,
            drives: vec![],
        };

        let yaml = r#"
        "#;
        let from_yaml: SataController = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let args: Vec<String> =
            vec!["-device ahci,id=ahci0,multifunction=on,bus=pci.0,addr=0".to_string()];
        assert_eq!(storage.get_qemu_args(0), args);
    }

    #[test]
    fn test_defaults_with_two_drives() {
        let storage = SataController {
            offset: 0,
            drives: vec![
                Drive::new("cd".to_string(), "drive0.img".to_string()),
                Drive::new("hd".to_string(), "drive1.img".to_string()),
            ],
        };

        let yaml = r#"
            drives:
            - type: "cd"
              file: "drive0.img"
            - type: "hd"
              file: "drive1.img"
        "#;
        let from_yaml: SataController = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let args: Vec<String> = vec![
            "-device ahci,id=ahci0,multifunction=on,bus=pci.0,addr=0".to_string(),
            "-drive id=drive-sata0,file=drive0.img,if=none,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device ide-cd,id=sata0,drive=drive-sata0,bus=ahci0.0"
                .to_string(),
            "-drive id=drive-sata1,file=drive1.img,if=none,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device ide-hd,id=sata1,drive=drive-sata1,bus=ahci0.1"
                .to_string(),
        ];
        assert_eq!(storage.get_qemu_args(0), args);
    }
}
