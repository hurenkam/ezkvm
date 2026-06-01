use std::path::PathBuf;

use crate::config_importer::{ConfigArgs, ConfigImportError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProxmoxImportOptions {
    pub(super) config_path: PathBuf,
}

impl ProxmoxImportOptions {
    pub(super) fn parse(config_args: ConfigArgs) -> Result<Self, ConfigImportError> {
        let (config_path_arg, extra_args) =
            config_args
                .args
                .split_first()
                .ok_or(ConfigImportError::MissingConfigPath {
                    importer: "proxmox",
                })?;

        if !extra_args.is_empty() {
            return Err(ConfigImportError::UnexpectedArgs {
                importer: "proxmox",
                args: extra_args.to_vec(),
            });
        }

        Ok(Self {
            config_path: PathBuf::from(config_path_arg),
        })
    }
}
