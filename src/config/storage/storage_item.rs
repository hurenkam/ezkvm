use crate::config::storage::storage_footer::StorageFooter;
use crate::config::storage::storage_header::StorageHeader;
use crate::config::storage::storage_payload::StoragePayload;
use crate::config::QemuDevice;
use derive_getters::Getters;
use serde::Deserialize;

#[derive(Deserialize, Debug, Getters)]
pub struct StorageItem {
    #[serde(flatten)]
    header: StorageHeader,
    #[serde(flatten)]
    payload: Box<dyn StoragePayload>,
    #[serde(flatten)]
    footer: StorageFooter,
}

impl QemuDevice for StorageItem {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let mut drive_args: Vec<String> = vec![];
        drive_args.extend(self.header().get_drive_options());
        drive_args.extend(self.payload().get_drive_options(index));
        drive_args.extend(self.footer().get_drive_options());

        let mut device_args: Vec<String> = vec![];
        device_args.extend(self.header().get_device_options());
        device_args.extend(self.payload().get_device_options(index));
        device_args.extend(self.footer().get_device_options());

        vec![
            format!("-drive {}", drive_args.join(",")),
            format!("-device {}", device_args.join(",")),
        ]
    }
}
