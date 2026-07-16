//use crate::runtime_model::{
//    Audio, Cpu, Display, GuestAgent, IdeDevice, Memory, PciDevice, PcieDevice, SataDevice,
//    ScsiDevice, Tpm, UsbDevice,
//};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Audio {}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum CpuModel {
    #[default]
    Host,
    Named {
        name: String,
    },
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, new)]
pub struct Cpu {
    model: CpuModel,
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

/*
impl Cpu {
    pub fn model(&self) -> &CpuModel {
        &self.model
    }

    pub fn cores(&self) -> u8 {
        self.cores
    }

    pub fn threads(&self) -> u8 {
        self.threads
    }

    pub fn sockets(&self) -> u8 {
        self.sockets
    }

    pub fn enabled_features(&self) -> &[String] {
        &self.enabled_features
    }

    pub fn disabled_features(&self) -> &[String] {
        &self.disabled_features
    }

    pub fn with_features(mut self, enabled: Vec<String>, disabled: Vec<String>) -> Self {
        self.enabled_features = enabled;
        self.disabled_features = disabled;
        self
    }

    fn normalized_cores(&self) -> u8 {
        self.cores.max(1)
    }

    fn normalized_threads(&self) -> u8 {
        self.threads.max(1)
    }

    fn normalized_sockets(&self) -> u8 {
        self.sockets.max(1)
    }

    pub fn total_vcpus(&self) -> u32 {
        self.normalized_sockets() as u32
            * self.normalized_cores() as u32
            * self.normalized_threads() as u32
    }
}
*/

//#[derive(Debug, Clone, Deserialize, Serialize)]
//pub struct Display {}

//#[derive(Debug, Clone, Deserialize, Serialize)]
//pub struct GuestAgent {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdeDevice {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PciDevice {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PcieDevice {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SataDevice {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScsiDevice {}

//#[derive(Debug, Clone, Deserialize, Serialize)]
//pub struct Tpm {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsbDevice {}
