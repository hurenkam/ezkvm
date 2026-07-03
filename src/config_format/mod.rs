//! Configuration format importers and exporters for translating between source representations.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/model-separation-pipeline.md

mod ezkvm;
mod libvirt;
mod proxmox;
mod qemu_cmd;

use crate::runtime_model::RuntimeModel;
pub use ezkvm::{EzkvmConfigFileStore, EzkvmConfigSchema, EzkvmRuntimeBuilder, EzkvmSchemaBuilder};
#[allow(unused_imports)]
pub use libvirt::{LibvirtInputArgs, LibvirtLoader, LibvirtOutputArgs, LibvirtSaver};
pub use proxmox::{
    ProxmoxOptions, ProxmoxRuntimeBuilder, ProxmoxSchemaBuilder, ProxmoxStorageConfig,
};
#[allow(unused_imports)]
pub use qemu_cmd::{QemuInputArgs, QemuLoader, QemuOutputArgs, QemuSaver};
//pub use stages::{RuntimeBuilder, SchemaBuilder};

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

/// Parses text input into a format-specific schema type.
#[allow(dead_code)]
pub trait Parser {
    type Schema;
    type Error;
    fn parse(&self, source: &str) -> Result<Self::Schema, Self::Error>;
}

/// Builds a canonical `RuntimeModel` from a format-specific schema.
#[allow(dead_code)]
pub trait RuntimeBuilder {
    type Schema;
    fn with_schema(self, schema: Self::Schema) -> Self;
    fn build(self) -> Result<RuntimeModel, String>;
}

/// Builds a format-specific schema from a canonical `RuntimeModel`.
#[allow(dead_code)]
pub trait SchemaBuilder {
    type Schema;
    fn with_runtime(self, runtime: RuntimeModel) -> Self;
    fn build(self) -> Result<Self::Schema, String>;
}

/// Marshals a format-specific schema into text output.
#[allow(dead_code)]
pub trait Marshaler {
    type Schema;
    type Error;
    fn marshal(&self, schema: &Self::Schema) -> Result<String, Self::Error>;
}

#[cfg(test)]
mod parity_tests;
