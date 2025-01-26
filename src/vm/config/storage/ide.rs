use super::drive::Drive;
use super::Controller;
use super::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct IdeController {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    drives: Vec<Drive>,
}

impl QemuDevice for IdeController {
    fn get_qemu_args(&self, _controller_index: usize) -> Vec<String> {
        let mut result = vec![];
        for (index, drive) in self.drives.iter().enumerate() {
            let mut drive_args: Vec<String> = vec![format!("id=drive-ide{}", index)];
            drive_args.extend(drive.get_drive_options());

            let mut device_args: Vec<String> = vec![
                format!("ide-{}", drive.get_drive_type()),
                format!("id=ide{}", index),
                format!("drive=drive-ide{}", index),
                format!("bus=ide.{}", index),
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

#[typetag::deserialize(name = "ide")]
impl Controller for IdeController {}

#[cfg(test)]
mod tests {
    use super::QemuDevice;
    use super::*;

    #[test]
    fn test_all_default_values() {
        let storage = IdeController {
            offset: 0,
            drives: vec![],
        };

        let yaml = r#"
        "#;
        let from_yaml: IdeController = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let args: Vec<String> = vec![];
        assert_eq!(storage.get_qemu_args(0), args);
    }

    #[test]
    fn test_defaults_with_two_drives() {
        let storage = IdeController {
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
        let from_yaml: IdeController = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let args: Vec<String> = vec![
            "-drive id=drive-ide0,file=drive0.img,if=none,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device ide-cd,id=ide0,drive=drive-ide0,bus=ide.0"
                .to_string(),
            "-drive id=drive-ide1,file=drive1.img,if=none,format=raw,cache=none,detect-zeroes=unmap".to_string(),
            "-device ide-hd,id=ide1,drive=drive-ide1,bus=ide.1"
                .to_string(),
        ];
        assert_eq!(storage.get_qemu_args(0), args);
    }
}
