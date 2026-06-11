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

use std::path::PathBuf;

pub use ezkvm::{EzkvmExporter, EzkvmImporter, EzkvmInputArgs, EzkvmOutputArgs};
pub use libvirt::{LibvirtExporter, LibvirtImporter, LibvirtInputArgs, LibvirtOutputArgs};
pub use options::{ExportOptions, ImportOptions};
pub use proxmox::{ProxmoxExporter, ProxmoxImporter, ProxmoxInputArgs, ProxmoxOutputArgs};
pub use qemu_cmd::{QemuExporter, QemuImporter, QemuInputArgs, QemuOutputArgs};

/// Runtime configuration used by all importer and exporter stages.
pub type RuntimeConfig = crate::runtime_config::RuntimeConfig;

/// Errors returned when an importer rejects input or fails while reading a source configuration.
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

/// Imports a source configuration into RuntimeConfig.
pub trait Importer {
    /// Converts importer-specific options into a validated runtime configuration.
    fn import(&self, args: ImportOptions) -> Result<RuntimeConfig, ImportError>;
}

/// Exports RuntimeConfig into a destination format.
pub trait Exporter {
    /// Writes the runtime configuration to the target format and returns the output path.
    fn export(&self, runtime: &RuntimeConfig, args: ExportOptions) -> Result<PathBuf, ExportError>;
}
