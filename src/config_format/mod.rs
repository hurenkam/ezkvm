//! Configuration format importers and exporters for translating between source representations.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/model-separation-pipeline.md

mod ezkvm;
mod libvirt;
mod options;
mod proxmox;
mod qemu_cmd;
mod stages;

use std::path::PathBuf;

use crate::runtime_model::RuntimeModel;
pub use ezkvm::{EzkvmConfigFileStore, EzkvmConfigSchema, EzkvmRuntimeBuilder, EzkvmSchemaBuilder};
pub use libvirt::{LibvirtExporter, LibvirtImporter};
pub use options::{ExportOptions, ImportOptions};
pub use proxmox::{ProxmoxSchemaBuilder,ProxmoxOptions};
pub use proxmox::{ProxmoxExporter, ProxmoxImporter};
pub use proxmox::{ProxmoxRuntimeBuilder, ProxmoxStorageConfig};
pub use qemu_cmd::{QemuExporter, QemuImporter};
pub use stages::{RuntimeBuilder, SchemaBuilder};

/// Errors returned when an importer rejects input or fails while reading a source configuration.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The caller selected an importer that does not match the provided options.
    #[error("invalid import format options for selected importer")]
    InvalidFormat,
    /// The requested importer is not implemented in this build.
    #[error("unsupported importer '{0}'")]
    UnsupportedImporter(String),
    /// The importer failed while reading, parsing, or validating source data.
    #[error("import failed: {0}")]
    ImportFailed(String),
}

/// Errors returned when an exporter rejects input or fails while writing an output file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The caller selected an exporter that does not match the provided options.
    #[error("invalid export format options for selected exporter")]
    InvalidFormat,
    /// The requested exporter is not implemented in this build.
    #[error("unsupported exporter '{0}'")]
    UnsupportedExporter(String),
    /// The exporter failed while rendering or writing output data.
    #[error("export failed: {0}")]
    ExportFailed(String),
}

/// Imports a source configuration directly into a RuntimeModel.
#[allow(dead_code)] // TODO: wire to CLI
pub trait Importer {
    /// Converts source configuration into a validated runtime model.
    fn import(&self, args: ImportOptions) -> Result<RuntimeModel, ImportError>;
}

/// Exports a RuntimeModel into a destination format.
#[allow(dead_code)] // TODO: wire to CLI
pub trait Exporter {
    /// Writes the runtime model to the target format and returns the output path.
    fn export(&self, runtime: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError>;
}

#[allow(dead_code)] // TODO: wire to CLI
pub trait RuntimeModelLoader {
    type Args;
    type Error;

    fn load(&self, args: Self::Args) -> Result<RuntimeModel, Self::Error>;
}

#[allow(dead_code)] // TODO: wire to CLI
pub trait RuntimeModelSaver {
    type Args;
    type Error;

    fn save(&self, runtime: RuntimeModel, args: Self::Args) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod parity_tests;
