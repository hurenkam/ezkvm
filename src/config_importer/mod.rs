//! Config importer stage module scaffold.
//!
//! Owns source-adapter orchestration and source -> runtime_config mapping boundaries.

use std::path::{Path, PathBuf};

use crate::runtime_config::{ConformanceError, RuntimeConfig as VmRuntimeConfig};
mod arg_parsing;
pub mod ezkvm;
pub mod libvirt;
pub mod proxmox;
pub mod qemu;

pub(crate) use arg_parsing::extract_config_path;
pub use ezkvm::EzkvmConfigImporter;
pub use libvirt::LibvirtConfigImporter;
pub use proxmox::ProxmoxConfigImporter;
pub use qemu::QemuConfigImporter;

pub type RuntimeConfig = VmRuntimeConfig;

#[derive(Debug, Clone, Default)]
pub struct ConfigArgs {
    pub args: Vec<String>,
}

impl ConfigArgs {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}

pub(crate) fn read_config_text(
    config_path: &Path,
    importer: &'static str,
) -> Result<String, ConfigImportError> {
    std::fs::read_to_string(config_path).map_err(|source| ConfigImportError::ReadConfigFailed {
        importer,
        config_path: config_path.to_path_buf(),
        source,
    })
}

pub trait ConfigImporter {
    type ConfigError;

    fn import_config(&self, config_args: ConfigArgs) -> Result<RuntimeConfig, Self::ConfigError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigImportError {
    #[error(transparent)]
    Conformance(#[from] ConformanceError),
    #[error("{importer} importer requires a config filename argument")]
    MissingConfigPath { importer: &'static str },
    #[error("{importer} importer could not read config file {config_path}: {source}")]
    ReadConfigFailed {
        importer: &'static str,
        config_path: PathBuf,
        source: std::io::Error,
    },
    #[error("{importer} importer received unexpected argument(s): {args:?}")]
    UnexpectedArgs {
        importer: &'static str,
        args: Vec<String>,
    },
    #[error("{importer} importer failed: {reason}")]
    Importer {
        importer: &'static str,
        reason: String,
    },
    #[error("{importer} importer is not implemented yet")]
    UnsupportedImporter { importer: &'static str },
}
