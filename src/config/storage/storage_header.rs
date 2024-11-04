use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct StorageHeader {
    file: String,
}

impl StorageHeader {
    pub fn get_drive_options(&self) -> Vec<String> {
        vec![format!("file={},if=none,aio=io_uring", self.file)]
    }

    pub fn get_device_options(&self) -> Vec<String> {
        vec![]
    }
}
