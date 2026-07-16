use std::fmt::{Display as FmtDisplay, Formatter};
use std::str::FromStr;

use derive_getters::Getters;
use derive_new::new;

use crate::config::ezkvm::ConfigSchema;
use crate::config::ezkvm::compact_yaml::{ToStyledYaml, emit_styled_yaml};
use crate::serde_yaml;

impl FmtDisplay for ConfigSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match serde_yaml::to_string(self) {
            Ok(serialized) => f.write_str(&serialized),
            Err(_) => f.write_str("--- failed to serialize ---"),
        }
    }
}

impl FromStr for ConfigSchema {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s).map_err(|e| format!("failed to parse YAML: {e}"))
    }
}

impl ConfigSchema {
    pub fn to_styled_compact_yaml(&self) -> Result<String, String> {
        let config_styled = self.to_styled_yaml();
        emit_styled_yaml(&config_styled)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct ConfigFileStore {
    pub config_dir: std::path::PathBuf,
}

#[allow(dead_code)]
impl ConfigFileStore {
    pub fn save_config(&self, vm_name: &str, config: ConfigSchema) -> Result<(), std::io::Error> {
        let file_path = self.config_dir.join(format!("{vm_name}.yaml"));
        std::fs::create_dir_all(&self.config_dir)?;
        //let content = config.to_string();
        let content = config
            .to_styled_compact_yaml()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&file_path, content)?;
        Ok(())
    }
    pub fn load_config(&self, vm_name: &str) -> Result<ConfigSchema, std::io::Error> {
        let file_path = self.config_dir.join(format!("{vm_name}.yaml"));
        let content = std::fs::read_to_string(&file_path)?;
        let config = ConfigSchema::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(config)
    }
}
