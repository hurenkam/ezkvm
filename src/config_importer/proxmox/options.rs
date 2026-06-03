use std::path::PathBuf;

use crate::config_importer::{ConfigArgs, ConfigImportError, extract_config_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProxmoxImportOptions {
    pub(super) config_path: PathBuf,
    pub(super) _storage_path: Option<PathBuf>,
}

impl ProxmoxImportOptions {
    pub(super) fn parse(config_args: ConfigArgs) -> Result<Self, ConfigImportError> {
        let (config_path, leftovers) = extract_config_path("proxmox", config_args)?;
        let mut storage_path: Option<PathBuf> = None;

        for token in leftovers {
            let Some((key, value)) = token.split_once('=') else {
                return Err(ConfigImportError::UnexpectedArgs {
                    importer: "proxmox",
                    args: vec![token],
                });
            };

            if key != "storage" || value.is_empty() || storage_path.is_some() {
                return Err(ConfigImportError::UnexpectedArgs {
                    importer: "proxmox",
                    args: vec![format!("{key}={value}")],
                });
            }

            storage_path = Some(PathBuf::from(value));
        }

        Ok(Self {
            config_path,
            _storage_path: storage_path,
        })
    }
}
