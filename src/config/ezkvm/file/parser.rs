use std::str::FromStr;
use crate::config::ezkvm::ConfigSchema;
use crate::serde_yaml;

impl FromStr for ConfigSchema {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s).map_err(|e| format!("failed to parse YAML: {e}"))
    }
}
