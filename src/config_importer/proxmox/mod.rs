//! Proxmox .conf file import adapter.
//!
//! Owns Proxmox-specific parsing and adaptation to runtime config specification.

use self::options::ProxmoxImportOptions;
use self::parsing::parse_source;
use super::{ConfigArgs, ConfigImportError, ConfigImporter, RuntimeConfig, read_config_text};

mod options;
mod parsing;

#[derive(Debug, Default)]
pub struct ProxmoxConfigImporter;

impl ConfigImporter for ProxmoxConfigImporter {
    type ConfigError = ConfigImportError;

    fn import_config(&self, config_args: ConfigArgs) -> Result<RuntimeConfig, Self::ConfigError> {
        let options = ProxmoxImportOptions::parse(config_args)?;
        let source_text = read_config_text(&options.config_path, "proxmox")?;

        parse_source(&source_text, &options.config_path).map_err(|error| {
            ConfigImportError::Importer {
                importer: "proxmox",
                reason: error.to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod corpus_tests;
