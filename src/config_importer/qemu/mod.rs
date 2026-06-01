//! QEMU command line importer stub.

use std::path::Path;

use super::{ConfigArgs, ConfigImportError, ConfigImporter, RuntimeConfig};

#[derive(Debug, Default)]
pub struct QemuConfigImporter;

impl ConfigImporter for QemuConfigImporter {
    type ConfigError = ConfigImportError;

    fn import_config(&self, _config_args: ConfigArgs) -> Result<RuntimeConfig, Self::ConfigError> {
        let config_path_arg = _config_args
            .args
            .first()
            .ok_or(ConfigImportError::MissingConfigPath { importer: "qemu" })?;
        let _config_path = Path::new(config_path_arg);
        let _extra_options = &_config_args.args[1..];

        Err(ConfigImportError::UnsupportedImporter { importer: "qemu" })
    }
}
