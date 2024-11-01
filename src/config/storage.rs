mod scsi_drive;
mod ide_cd;

use serde::{Deserialize, Serialize};
use crate::config::{QemuDevice};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct StorageData {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    boot_index: Option<u8>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_drive_options: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_device_options: Vec<String>,
}
impl StorageData {
    pub fn drive(&self,child_args: Vec<String>) -> String {
        let mut args: Vec<String> = vec![
            format!("file={},if=none,aio=io_uring",self.file)
        ];
        args.extend(child_args);
        args.extend(self.extra_drive_options.clone());
        format!("-drive {}",args.join(","))
    }
    pub fn device(&self,child_args: Vec<String>) -> String {
        let mut args: Vec<String> = vec![];
        match self.boot_index {
            Some(index) => { args.extend(vec![format!("bootindex={}",index)]) }
            None => {}
        }
        args.extend(child_args);
        args.extend(self.extra_drive_options.clone());
        format!("-device {}",args.join(","))
    }
}

#[typetag::deserialize(tag = "type")]
pub trait Storage: QemuDevice {}

/*
#[derive(Serialize,Deserialize, PartialEq, Debug, Clone)]
pub struct Generic {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    boot_index: Option<u8>,
}
impl Generic {
    pub fn device_options(&self) -> Vec<String> {
        let boot_index = match self.boot_index.clone() {
            None => "".to_string(),
            Some(boot_index) => format!("bootindex={}", boot_index),
        };
        vec![
            format!("-device if=none,{}",boot_index)
        ]
    }

    pub fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![
            format!("-drive file={}",self.file)
        ]
    }
}

#[derive(Serialize,Deserialize, PartialEq, Debug, Clone, Getters)]
pub struct Extras {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    drive_options: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    device_options: Vec<String>,
}
*/




/*
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(tag="type")]
#[serde(rename_all = "snake_case")]
enum StorageDevice {
    IdeCd(IdeCd),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_defaults() {
        let storage = StorageDevice::IdeCd(IdeCd {
            generic: Generic {
                file: "file".to_string(),
                boot_index: None
            },
            media: "cdrom".to_string(),
            extras: Extras {
                drive_options: vec![],
                device_options: vec![]
            }
        });

        let yaml = r#"
            type: ide_cd
            file: file
            media: cdrom
        "#;

        let to_yaml = serde_yaml::to_string(&storage).unwrap();
        let from_yaml: StorageDevice = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(from_yaml,storage);
        assert_eq!(to_yaml,"type: ide_cd\nfile: file\nmedia: cdrom\n".to_string());
    }

    #[test]
    fn test_valid_values() {
        let storage = StorageDevice::IdeCd(IdeCd {
            generic: Generic {
                file: "file".to_string(),
                boot_index: Some(0)
            },
            media: "cdrom".to_string(),
            extras: Extras {
                drive_options: vec!["drive_option_1".to_string(),"drive_option_2".to_string()],
                device_options: vec!["device_option_3".to_string()]
            }
        });

        let yaml = r#"
            type: ide_cd
            file: file
            boot_index: 0
            media: cdrom

            drive_options:
            - drive_option_1
            - drive_option_2

            device_options:
            - device_option_3
        "#;

        let to_yaml = serde_yaml::to_string(&storage).unwrap();
        let from_yaml: StorageDevice = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(from_yaml,storage);
        assert_eq!(to_yaml,"type: ide_cd\nfile: file\nboot_index: 0\nmedia: cdrom\ndrive_options:\n- drive_option_1\n- drive_option_2\ndevice_options:\n- device_option_3\n".to_string());
    }
}
*/
