use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, new)]
pub struct MemorySchema {
    size: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hugepages_kb: Option<usize>,
    #[serde(default)]
    numa_enabled: bool,
}
