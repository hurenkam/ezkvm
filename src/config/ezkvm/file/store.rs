
use std::str::FromStr;

use derive_getters::Getters;
use derive_new::new;

use crate::config::ezkvm::ConfigSchema;

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
