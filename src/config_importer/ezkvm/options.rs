use std::path::PathBuf;

use crate::config_importer::{ConfigArgs, ConfigImportError, extract_config_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EzkvmImportOptions {
    pub(super) config_path: PathBuf,
    pub(super) _host_path: Option<PathBuf>,
    pub(super) _profiles_path: Option<PathBuf>,
}

impl EzkvmImportOptions {
    pub(super) fn parse(config_args: ConfigArgs) -> Result<Self, ConfigImportError> {
        let (config_path, leftovers) = extract_config_path("ezkvm", config_args)?;
        let mut host_path: Option<PathBuf> = None;
        let mut profiles_path: Option<PathBuf> = None;

        for token in leftovers {
            let Some((key, value)) = token.split_once('=') else {
                return Err(ConfigImportError::UnexpectedArgs {
                    importer: "ezkvm",
                    args: vec![token],
                });
            };

            if value.is_empty() {
                return Err(ConfigImportError::UnexpectedArgs {
                    importer: "ezkvm",
                    args: vec![format!("{key}=")],
                });
            }

            match key {
                "host" => {
                    if host_path.is_some() {
                        return Err(ConfigImportError::UnexpectedArgs {
                            importer: "ezkvm",
                            args: vec![format!("{key}={value}")],
                        });
                    }

                    host_path = Some(PathBuf::from(value));
                }
                "profiles" => {
                    if profiles_path.is_some() {
                        return Err(ConfigImportError::UnexpectedArgs {
                            importer: "ezkvm",
                            args: vec![format!("{key}={value}")],
                        });
                    }

                    profiles_path = Some(PathBuf::from(value));
                }
                _ => {
                    return Err(ConfigImportError::UnexpectedArgs {
                        importer: "ezkvm",
                        args: vec![format!("{key}={value}")],
                    });
                }
            }
        }

        Ok(Self {
            config_path,
            _host_path: host_path,
            _profiles_path: profiles_path,
        })
    }
}
