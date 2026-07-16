use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::config::ezkvm::schema::chipset::ChipsetSchema;

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct MachineSchema {
    #[serde(flatten)]
    chipset: ChipsetSchema,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}
