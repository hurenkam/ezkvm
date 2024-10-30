mod looking_glass;
mod qemu_device;
mod spice;
mod system;

use crate::resource::lock::EzkvmError;
use crate::yaml::display::Display;
use crate::yaml::general::General;
use crate::yaml::gpu::Gpu;
use crate::yaml::host::Host;
use crate::yaml::network::Network;
use crate::yaml::storage::Storage;
use derive_getters::Getters;
use log::info;
use serde::{Deserialize, Deserializer};

pub use looking_glass::LookingGlass;
pub use qemu_device::QemuDevice;
pub use spice::Spice;
pub use system::System;

#[derive(Deserialize, Debug, Default, Getters)]
pub struct Config {
    #[serde(default, deserialize_with = "default_when_missing")]
    general: General,
    #[serde(default, deserialize_with = "default_when_missing")]
    system: System,
    #[serde(default, deserialize_with = "default_when_missing")]
    display: Option<Display>,
    #[serde(default, deserialize_with = "default_when_missing")]
    gpu: Option<Gpu>,
    #[serde(default, deserialize_with = "default_when_missing")]
    spice: Option<Spice>,
    #[serde(default, deserialize_with = "default_when_missing")]
    looking_glass: Option<LookingGlass>,
    #[serde(default, deserialize_with = "default_when_missing")]
    host: Option<Host>,
    #[serde(default, deserialize_with = "default_when_missing")]
    storage: Vec<Storage>,
    #[serde(default, deserialize_with = "default_when_missing")]
    network: Vec<Network>,
}

impl Config {
    pub(crate) fn allocate_resources(&self) -> Result<Vec<String>, EzkvmError> {
        Ok(vec![])
    }
}

impl QemuDevice for Config {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.general.get_qemu_args(0));
        result.extend(self.system.get_qemu_args(0));

        match self.display() {
            None => {}
            Some(display) => {
                result.extend(display.get_qemu_args(0));
            }
        }

        match self.gpu() {
            None => {}
            Some(gpu) => {
                result.extend(gpu.get_qemu_args(0));
            }
        }

        match self.spice() {
            None => {}
            Some(spice) => {
                result.extend(spice.get_qemu_args(0));
            }
        }

        match self.looking_glass() {
            None => {}
            Some(looking_glass_host) => {
                result.extend(looking_glass_host.get_qemu_args(0));
            }
        }

        match self.host() {
            None => {}
            Some(host) => {
                result.extend(host.get_qemu_args(0));
            }
        }

        for (i, disk) in self.storage.iter().enumerate() {
            result.extend(disk.get_qemu_args(i));
        }

        for (i, network) in self.network.iter().enumerate() {
            result.extend(network.get_qemu_args(i));
        }

        let mut args = "qemu-system-x86_64".to_string();
        for arg in result {
            info!("{}", arg);
            args = format!("{} {}", args, arg).to_string();
        }

        args.split_whitespace().map(str::to_string).collect()
    }

    fn pre_start(&self, config: &Config) {
        self.system.pre_start(config);
    }

    fn post_start(&self, config: &Config) {
        self.system.post_start(config);

        match &self.looking_glass {
            None => {}
            Some(looking_glass_host) => {
                looking_glass_host.post_start(config);
            }
        }
    }
}

pub fn default_when_missing<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let option = Option::deserialize(deserializer)?;
    Ok(option.unwrap_or_default())
}
