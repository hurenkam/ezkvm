use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct StorageFooter {
    #[serde(skip_serializing_if = "Option::is_none")]
    boot_index: Option<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_drive_options: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_device_options: Vec<String>,
}

impl StorageFooter {
    pub fn get_drive_options(&self) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.extra_drive_options.clone());
        result
    }

    pub fn get_device_options(&self) -> Vec<String> {
        let mut result = vec![];
        if let Some(boot_index) = self.boot_index {
            result.push(format!("bootindex={}", boot_index));
        }
        result.extend(self.extra_device_options.clone());
        result
    }
}
