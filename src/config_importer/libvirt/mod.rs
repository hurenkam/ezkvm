//! Libvirt XML importer stub.

use std::path::PathBuf;

use super::{ConfigArgs, ConfigImportError, ConfigImporter, RuntimeConfig, extract_config_path};

#[derive(Debug, Default)]
pub struct LibvirtConfigImporter;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LibvirtImportOptions {
    _config_path: PathBuf,
}

impl LibvirtImportOptions {
    fn parse(config_args: ConfigArgs) -> Result<Self, ConfigImportError> {
        let (config_path, leftovers) = extract_config_path("libvirt", config_args)?;

        if !leftovers.is_empty() {
            return Err(ConfigImportError::UnexpectedArgs {
                importer: "libvirt",
                args: leftovers,
            });
        }

        Ok(Self {
            _config_path: config_path,
        })
    }
}

impl ConfigImporter for LibvirtConfigImporter {
    type ConfigError = ConfigImportError;

    fn import_config(&self, config_args: ConfigArgs) -> Result<RuntimeConfig, Self::ConfigError> {
        let _options = LibvirtImportOptions::parse(config_args)?;

        Err(ConfigImportError::UnsupportedImporter {
            importer: "libvirt",
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::config_importer::{ConfigArgs, ConfigImportError};

    #[test]
    fn parse_options_requires_config_path() {
        let error = super::LibvirtImportOptions::parse(ConfigArgs::new(vec![]))
            .expect_err("missing path must fail");

        assert!(matches!(
            error,
            ConfigImportError::MissingConfigPath {
                importer: "libvirt"
            }
        ));
    }

    #[test]
    fn parse_options_rejects_extra_args() {
        let error = super::LibvirtImportOptions::parse(ConfigArgs::new(vec![
            "config=domain.xml".to_string(),
            "profiles=/etc/libvirt/profiles.d".to_string(),
        ]))
        .expect_err("extra args must fail");

        assert!(matches!(
            error,
            ConfigImportError::UnexpectedArgs {
                importer: "libvirt",
                ..
            }
        ));
    }
}
