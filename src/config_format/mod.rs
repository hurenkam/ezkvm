mod ezkvm;
mod libvirt;
mod options;
mod proxmox;
mod qemu_cmd;

use std::path::PathBuf;

pub use ezkvm::{EzkvmExporter, EzkvmImporter, EzkvmInputArgs, EzkvmOutputArgs};
pub use libvirt::{LibvirtExporter, LibvirtImporter, LibvirtInputArgs, LibvirtOutputArgs};
pub use options::{ExportOptions, ImportOptions};
pub use proxmox::{ProxmoxExporter, ProxmoxImporter, ProxmoxInputArgs, ProxmoxOutputArgs};
pub use qemu_cmd::{QemuExporter, QemuImporter, QemuInputArgs, QemuOutputArgs};

pub type RuntimeConfig = crate::runtime_config::RuntimeConfig;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("invalid import format options for selected importer")]
    InvalidFormat,
    #[error("unsupported importer '{0}'")]
    UnsupportedImporter(String),
    #[error("import failed: {0}")]
    ImportFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("invalid export format options for selected exporter")]
    InvalidFormat,
    #[error("unsupported exporter '{0}'")]
    UnsupportedExporter(String),
    #[error("export failed: {0}")]
    ExportFailed(String),
}

pub trait Importer {
    fn import(&self, args: ImportOptions) -> Result<RuntimeConfig, ImportError>;
}

pub trait Exporter {
    fn export(&self, runtime: &RuntimeConfig, args: ExportOptions) -> Result<PathBuf, ExportError>;
}
