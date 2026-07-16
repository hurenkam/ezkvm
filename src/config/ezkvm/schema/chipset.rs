use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct Q35ChipsetSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct I440FXChipsetSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChipsetSchema {
    Q35 { q35: Q35ChipsetSchema },
    I440FX { i440fx: I440FXChipsetSchema },
}
