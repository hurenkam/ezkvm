use crate::config::{CentralConfig, DEFAULT_CENTRAL_CONFIG_PATHS};
use std::path::Path;

impl CentralConfig {
    /// Load central configuration from the default location or environment variable
    pub fn load() -> anyhow::Result<Self> {
        if let Ok(config_path) = std::env::var("EZKVM_CONFIG") {
            return if Path::new(&config_path).exists() {
                Self::from_file(&config_path)
            } else {
                Ok(Self::default())
            };
        }

        for config_path in DEFAULT_CENTRAL_CONFIG_PATHS {
            if Path::new(config_path).exists() {
                return Self::from_file(config_path);
            }
        }

        Ok(Self::default())
    }

    /// Load central configuration from a specific file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: CentralConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
