use std::cmp::PartialEq;
use std::fmt;
use log::info;
use serde::{de, Deserialize, Deserializer};
use serde::de::{MapAccess, Visitor};
use crate::resource::lock::EzkvmError;
use crate::yaml::display::Display;
use crate::yaml::general::General;
use crate::yaml::gpu::Gpu;
use crate::yaml::host::Host;
use crate::yaml::looking_glass::LookingGlass;
use crate::yaml::network::Network;
use crate::yaml::{SwtpmArgs,QemuArgs,LgClientArgs};
use crate::yaml::spice::Spice;
use crate::yaml::storage::Storage;
use crate::yaml::system::System;

#[derive(Debug)]
pub struct Config {
    general: General,
    system: System,
    display: Option<Display>,
    gpu: Option<Gpu>,
    spice: Option<Spice>,
    looking_glass: Option<LookingGlass>,
    host: Option<Host>,
    storage: Vec<Storage>,
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
        return self.looking_glass != None
    }

    pub fn get_display(&self) -> Option<Display> {
        return self.display.clone()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General::default(),
            system: System::default(),
            display: None,
            gpu: None,
            spice: None,
            looking_glass: None,
            host: None,
            storage: vec![],
            network: vec![],
        }
    }
}

impl SwtpmArgs for Config {
    fn get_swtpm_args(&self, index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.system.get_swtpm_args(0));
        result
    }
}

impl QemuArgs for Config {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
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

        for (i,disk) in self.storage.iter().enumerate() {
            result.extend(disk.get_qemu_args(i));
        }

        for (i,network) in self.network.iter().enumerate() {
            result.extend(network.get_qemu_args(i));
        }

        let mut args = "qemu-system-x86_64".to_string();
        for arg in result {
            info!("{}",arg);
            args = format!("{} {}",args,arg).to_string();
        }

        args.split_whitespace().map(str::to_string).collect()
    }
}

impl LgClientArgs for Config {
    fn get_lg_client_args(&self, index: usize) -> Vec<String> {
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

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            General,
            System,
            Display,
            Gpu,
            Spice,
            LookingGlass,
            Host,
            Storage,
            Network
        }

        struct ConfigVisitor;

        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = Config;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct YamlConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Config, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let mut config = Config::default();
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::General => {
                            config.general = map.next_value()?;
                        }
                        Field::System => {
                            config.system = map.next_value()?;
                        }
                        Field::Display => {
                            config.display = map.next_value()?;
                        }
                        Field::Gpu => {
                            config.gpu = map.next_value()?;
                        }
                        Field::Spice => {
                            config.spice = map.next_value()?;
                        }
                        Field::LookingGlass => {
                            config.looking_glass = map.next_value()?;
                        }
                        Field::Host => {
                            config.host = map.next_value()?;
                        }
                        Field::Storage => {
                            config.storage = map.next_value()?;
                        }
                        Field::Network => {
                            config.network = map.next_value()?;
                        }
                    }
                }

                Ok(config)
            }
        }

        const FIELDS: &[&str] = &["general", "system", "display", "gpu", "spice", "looking_glass_host", "host", "storage", "network"];
        deserializer.deserialize_struct("Config", FIELDS, ConfigVisitor)
    }
}
