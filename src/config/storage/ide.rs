use crate::config::storage::storage_payload::StoragePayload;
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum IdeDeviceType {
    Cd,
    Hd,
    Ssd,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Ide {
    device_type: IdeDeviceType,
    //#[serde(default)]
    //discard: Option<String>,
    //#[serde(default = "Ide::cache_default")]
    //cache: String,
    //#[serde(default = "Ide::format_default")]
    //format: String,
    //#[serde(default = "Ide::detect_zeroes_default")]
    //detect_zeroes: String,
    #[serde(default = "Ide::bus_default")]
    bus: String,
    //#[serde(default = "Ide::rotation_rate_default")]
    //rotation_rate: u8,
    #[serde(default = "Ide::unit_default")]
    unit: String,
    #[serde(default = "Ide::media_default")]
    media: String,
}

impl Ide {
    //optional_value_getter!(discard("discard"): String);
    //required_value_getter!(cache("cache"): String = "none".to_string());
    //required_value_getter!(format("format"): String = "raw".to_string());
    //required_value_getter!(detect_zeroes("detect-zeroes"): String = "unmap".to_string());
    required_value_getter!(bus("bus"): String = "ide.0".to_string());
    //required_value_getter!(rotation_rate("rotation_rate"): u8 = 1);
    required_value_getter!(unit("unit"): String = "0".to_string());
    required_value_getter!(media("media"): String = "cdrom".to_string());

    fn device_type(&self) -> String {
        match self.device_type {
            IdeDeviceType::Cd => "cd".to_string(),
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
}

#[typetag::deserialize(name = "ide")]
impl StoragePayload for Ide {
    fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "id=drive-ide{}",
            index,
            //self.discard(),
            //self.format(),
            //self.cache(),
            //self.detect_zeroes()
        )]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        //"ide-cd,bus=ide.{},drive=drive-ide{},id=ide{},unit={}",
        //  index, index, index, self.unit
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
            device_type: IdeDeviceType::Cd,
            bus: "ide.0".to_string(),
            unit: "0".to_string(),
            media: "cdrom".to_string(),
        };

        let yaml = r#"
            type: "ide"
            device_type: "cd"
            file: "default_file"
        "#;
        let from_yaml: Ide = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec!["id=ide0".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["ide-cd,bus=ide.0,drive=drive-ide0,id=ide0,unit=0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=ide5".to_string(),
            "-device ide-cd,bus=ide.0,drive=drive-ide5,id=ide5,unit=0".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    // -drive
    //      file=/home/hurenkam/Workspace/ezkvm.stable/debian-30r6-dvd-i386-binary-1.iso,
    //      if=none,
    //      aio=io_uring,
    //      id=ide0
    // -device
    //      ide-cd,
    //      bus=ide.0,
    //      drive=drive-ide0,
    //      id=ide0,
    //      unit=0
    #[test]
    fn test_ide_cd() {
        let storage = Ide {
            device_type: IdeDeviceType::Cd,
            bus: "ide.0".to_string(),
            unit: "0".to_string(),
            media: "cdrom".to_string(),
        };

        let yaml = r#"
            type: "ide"
            device_type: "cd"
            file: "default_file"
        "#;
        let from_yaml: Ide = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec!["id=ide0".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["ide-cd,bus=ide.0,drive=drive-ide0,id=ide0,unit=0".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=ide5".to_string(),
            "-device ide-cd,bus=ide.0,drive=drive-ide5,id=ide5,unit=0".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    #[test]
    fn test_ide_hd() {
        let storage = Ide {
            device_type: IdeDeviceType::Hd,
            bus: "ide.0".to_string(),
            unit: "1".to_string(),
            media: "cdrom".to_string(),
        };

        let yaml = r#"
            type: "ide"
            device_type: "hd"
            file: "default_file"
            unit: "1"
        "#;
        let from_yaml: Ide = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(storage, from_yaml);

        let drive_args: Vec<String> = vec!["id=ide0".to_string()];
        assert_eq!(storage.get_drive_options(0), drive_args);

        let device_args: Vec<String> =
            vec!["ide-hd,bus=ide.0,drive=drive-ide0,id=ide0,unit=1".to_string()];
        assert_eq!(storage.get_device_options(0), device_args);

        let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
        let expected: Vec<String> = vec![
            "-drive file=default_file,if=none,aio=io_uring,id=ide5".to_string(),
            "-device ide-hd,bus=ide.0,drive=drive-ide5,id=ide5,unit=1".to_string(),
        ];

        assert_eq!(from_yaml.get_qemu_args(5), expected);
    }

    /*
       #[test]
       fn test_all_default_values() {
           let storage = ScsiHd {
               discard: None,
               cache: ScsiHd::cache_default(),
               format: ScsiHd::format_default(),
               detect_zeroes: ScsiHd::detect_zeroes_default(),
               bus: ScsiHd::bus_default(),
               rotation_rate: 1,
           };

           let yaml = r#"
               type: "scsi-hd"
               file: "default_file"
           "#;
           let from_yaml: ScsiHd = serde_yaml::from_str(yaml).unwrap();
           assert_eq!(storage, from_yaml);

           let drive_args: Vec<String> =
               vec!["id=drive-scsi0,format=raw,cache=none,detect-zeroes=unmap".to_string()];
           assert_eq!(storage.get_drive_options(0), drive_args);

           let device_args: Vec<String> = vec![
               "scsi-hd,scsi-id=0,drive=drive-scsi0,id=scsi0,bus=scsihw0.0,rotation_rate=1"
                   .to_string(),
           ];
           assert_eq!(storage.get_device_options(0), device_args);

           let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
           let expected: Vec<String> = vec![
               "-drive file=default_file,if=none,aio=io_uring,id=drive-scsi5,format=raw,cache=none,detect-zeroes=unmap".to_string(),
               "-device scsi-hd,scsi-id=5,drive=drive-scsi5,id=scsi5,bus=scsihw0.0,rotation_rate=1".to_string()
           ];

           assert_eq!(from_yaml.get_qemu_args(5), expected);
       }

       #[test]
       fn test_all_valid_values() {
           let storage = ScsiHd {
               discard: Some("on".to_string()),
               cache: "write-back".to_string(),
               format: "qcow2".to_string(),
               detect_zeroes: "off".to_string(),
               bus: "scsihw1.2".to_string(),
               rotation_rate: 3,
           };

           let yaml = r#"
               type: "scsi-hd"
               file: "valid_file"
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
           let from_yaml: ScsiHd = serde_yaml::from_str(yaml).unwrap();
           assert_eq!(storage, from_yaml);

           let drive_args: Vec<String> = vec![
               "id=drive-scsi5,discard=on,format=qcow2,cache=write-back,detect-zeroes=off".to_string(),
           ];
           assert_eq!(storage.get_drive_options(5), drive_args);

           let device_args: Vec<String> = vec![
               "scsi-hd,scsi-id=5,drive=drive-scsi5,id=scsi5,bus=scsihw1.2,rotation_rate=3"
                   .to_string(),
           ];
           assert_eq!(storage.get_device_options(5), device_args);

           let from_yaml: StorageItem = serde_yaml::from_str(yaml).unwrap();
           let expected: Vec<String> = vec![
               "-drive file=valid_file,if=none,aio=io_uring,id=drive-scsi5,discard=on,format=qcow2,cache=write-back,detect-zeroes=off,option_1,option_2".to_string(),
               "-device scsi-hd,scsi-id=5,drive=drive-scsi5,id=scsi5,bus=scsihw1.2,rotation_rate=3,bootindex=1,option_3".to_string()
           ];

           assert_eq!(from_yaml.get_qemu_args(5), expected);
       }

    */
}
