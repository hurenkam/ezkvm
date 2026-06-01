use std::path::PathBuf;

use crate::config_importer::{ConfigArgs, ConfigImportError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EzkvmImportOptions {
    pub(super) config_path: PathBuf,
}

impl EzkvmImportOptions {
    pub(super) fn parse(config_args: ConfigArgs) -> Result<Self, ConfigImportError> {
        let (config_path_arg, extra_args) = config_args
            .args
            .split_first()
            .ok_or(ConfigImportError::MissingConfigPath { importer: "ezkvm" })?;

        if !extra_args.is_empty() {
            return Err(ConfigImportError::UnexpectedArgs {
                importer: "ezkvm",
                args: extra_args.to_vec(),
            });
        }

        Ok(Self {
            config_path: PathBuf::from(config_path_arg),
        })
    }
}
