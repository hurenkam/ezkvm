mod ezkvm;
mod libvirt;
mod proxmox;
mod qemu_cmd;

use std::path::PathBuf;

#[cfg(test)]
pub(crate) use ezkvm::validate_ezkvm_config;
pub use ezkvm::{EzkvmExporter, EzkvmImporter, EzkvmInputArgs, EzkvmOutputArgs};
pub use libvirt::{LibvirtExporter, LibvirtImporter, LibvirtInputArgs, LibvirtOutputArgs};
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ImportOptions {
    Ezkvm { host: String, vm: String },
    Proxmox { storage: String, vm: String },
    Qemu { vm: String },
    Libvirt { vm: String },
}
impl ImportOptions {
    pub fn import_runtime_config(&self) -> Result<RuntimeConfig, String> {
        match self {
            ImportOptions::Ezkvm { .. } => EzkvmImporter.import(self.clone()),
            ImportOptions::Proxmox { .. } => ProxmoxImporter.import(self.clone()),
            ImportOptions::Qemu { .. } => QemuImporter.import(self.clone()),
            ImportOptions::Libvirt { .. } => LibvirtImporter.import(self.clone()),
        }
        .map_err(|error| format!("import failed: {error}"))
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ExportOptions {
    Ezkvm { host: String, vm: Option<String> },
    Proxmox { storage: String, vm: String },
    Qemu { vm: Option<String> },
    Libvirt { vm: String },
}
impl ExportOptions {
    pub fn export_runtime(&self, runtime: &RuntimeConfig) -> Result<PathBuf, String> {
        match self {
            ExportOptions::Ezkvm { .. } => EzkvmExporter.export(runtime, self.clone()),
            ExportOptions::Proxmox { .. } => ProxmoxExporter.export(runtime, self.clone()),
            ExportOptions::Qemu { .. } => QemuExporter.export(runtime, self.clone()),
            ExportOptions::Libvirt { .. } => LibvirtExporter.export(runtime, self.clone()),
        }
        .map_err(|error| format!("export failed: {error}"))
    }
}