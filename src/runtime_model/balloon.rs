use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct BalloonConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    free_page_reporting: bool,
}
