use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct SerialConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    socket_path: Option<String>,
    #[serde(default)]
    server: bool,
    #[serde(default)]
    wait: bool,
}
