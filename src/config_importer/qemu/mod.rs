//! QEMU command line importer stub.

use std::path::PathBuf;

use super::{ConfigArgs, ConfigImportError, ConfigImporter, RuntimeConfig};

#[derive(Debug, Default)]
pub struct QemuConfigImporter;

#[derive(Debug, Clone, PartialEq, Eq)]
struct QemuImportOptions {
    _config_path: PathBuf,
}

impl QemuImportOptions {
    fn parse(config_args: ConfigArgs) -> Result<Self, ConfigImportError> {
        let (config_path_arg, extra_args) = config_args
            .args
            .split_first()
            .ok_or(ConfigImportError::MissingConfigPath { importer: "qemu" })?;

        if !extra_args.is_empty() {
            return Err(ConfigImportError::UnexpectedArgs {
                importer: "qemu",
                args: extra_args.to_vec(),
            });
        }

        Ok(Self {
            _config_path: PathBuf::from(config_path_arg),
        })
    }
}

impl ConfigImporter for QemuConfigImporter {
    type ConfigError = ConfigImportError;

    fn import_config(&self, config_args: ConfigArgs) -> Result<RuntimeConfig, Self::ConfigError> {
        let _options = QemuImportOptions::parse(config_args)?;

        Err(ConfigImportError::UnsupportedImporter { importer: "qemu" })
    }
}

#[cfg(test)]
mod tests {
    use crate::config_importer::{ConfigArgs, ConfigImportError};

    #[test]
    fn parse_options_requires_config_path() {
        let error = super::QemuImportOptions::parse(ConfigArgs::new(vec![]))
            .expect_err("missing path must fail");

        assert!(matches!(
            error,
            ConfigImportError::MissingConfigPath { importer: "qemu" }
        ));
    }

    #[test]
    fn parse_options_rejects_extra_args() {
        let error = super::QemuImportOptions::parse(ConfigArgs::new(vec![
            "capture.txt".to_string(),
            "--unexpected".to_string(),
        ]))
        .expect_err("extra args must fail");

        assert!(matches!(
            error,
            ConfigImportError::UnexpectedArgs {
                importer: "qemu",
                ..
            }
        ));
    }
}
