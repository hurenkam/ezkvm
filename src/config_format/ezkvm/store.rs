use std::str::FromStr;

use derive_getters::Getters;
use derive_new::new;

use crate::config_format::EzkvmConfigSchema;

#[derive(Debug, Clone, Getters, new)]
pub struct EzkvmConfigFileStore {
    pub config_dir: std::path::PathBuf,
}

impl EzkvmConfigFileStore {
    pub fn save_config(
        &self,
        vm_name: &str,
        config: EzkvmConfigSchema,
    ) -> Result<(), std::io::Error> {
        let file_path = self.config_dir.join(format!("{vm_name}.yaml"));
        std::fs::create_dir_all(&self.config_dir)?;
        //let content = config.to_string();
        let content = config
            .to_styled_compact_yaml()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&file_path, content)?;
        Ok(())
    }
    pub fn load_config(&self, vm_name: &str) -> Result<EzkvmConfigSchema, std::io::Error> {
        let file_path = self.config_dir.join(format!("{vm_name}.yaml"));
        let content = std::fs::read_to_string(&file_path)?;
        let config = EzkvmConfigSchema::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(config)
    }
}
