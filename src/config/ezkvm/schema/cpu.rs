use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum CpuModelSchema {
    #[default]
    Host,
    Named {
        name: String,
    },
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, new)]
pub struct CpuSchema {
    model: CpuModelSchema,
    cores: u8,
    threads: u8,
    sockets: u8,
    #[serde(default)]
    #[new(default)]
    enabled_features: Vec<String>,
    #[serde(default)]
    #[new(default)]
    disabled_features: Vec<String>,
}
