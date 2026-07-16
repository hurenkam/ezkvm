use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::config::ezkvm::schema::AudioSchema;
use crate::config::ezkvm::schema::DisplaySchema;
use crate::config::ezkvm::schema::ResourceSchema;

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct HostSchema {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    display: Option<DisplaySchema>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    audio: Option<AudioSchema>,
    resources: Vec<ResourceSchema>,
}
