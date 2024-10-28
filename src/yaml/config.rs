use crate::resource::lock::EzkvmError;
use crate::yaml::display::Display;
use crate::yaml::general::General;
use crate::yaml::gpu::Gpu;
use crate::yaml::host::Host;
use crate::yaml::looking_glass::LookingGlass;
use crate::yaml::network::Network;
use crate::yaml::spice::Spice;
use crate::yaml::storage::Storage;
use crate::yaml::system::System;
use crate::yaml::{LgClientArgs, QemuArgs, SwtpmArgs};
use log::info;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug, Default)]
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

    pub fn has_tpm(&self) -> bool {
        self.system.get_tpm() != None
    }

    pub fn has_lg(&self) -> bool {
        return self.looking_glass != None;
    }

    pub fn get_display(&self) -> Option<Display> {
        return self.display.clone();
    }
}

impl SwtpmArgs for Config {
    fn get_swtpm_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.system.get_swtpm_args(0));
        result
    }
}

impl QemuArgs for Config {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.general.get_qemu_args(0));
        result.extend(self.system.get_qemu_args(0));

        match &self.display {
            None => {}
            Some(display) => {
                result.extend(display.get_qemu_args(0));
            }
        }

        match &self.gpu {
            None => {}
            Some(gpu) => {
                result.extend(gpu.get_qemu_args(0));
            }
        }

        match &self.spice {
            None => {}
            Some(spice) => {
                result.extend(spice.get_qemu_args(0));
            }
        }

        match &self.looking_glass {
            None => {}
            Some(looking_glass_host) => {
                result.extend(looking_glass_host.get_qemu_args(0));
            }
        }

        match &self.host {
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
}

impl LgClientArgs for Config {
    fn get_lg_client_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];

        match &self.spice {
            None => {}
            Some(spice) => {
                result.extend(spice.get_lg_client_args(0));
            }
        }

        match &self.looking_glass {
            None => {}
            Some(looking_glass) => {
                result.extend(looking_glass.get_lg_client_args(0));
            }
        }

        result
    }
}

fn default_when_missing<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let option = Option::deserialize(deserializer)?;
    Ok(option.unwrap_or_default())
}
