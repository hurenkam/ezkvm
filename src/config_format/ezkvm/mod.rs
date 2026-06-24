//! ezkvm source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

mod builder;
mod diagnostics;
mod errors;
mod exporter;
mod importer;
mod marshaler;
mod parser;
mod renderer;
mod runtime_builder;
mod schema;
mod schema_builder;
mod validation;
mod virtual_machine;

pub use errors::{ParseError, ValidationIssue};
pub use exporter::EzkvmOutputArgs;
pub use importer::EzkvmInputArgs;
pub use marshaler::EzkvmMarshaler;
pub use parser::EzkvmParser;
pub use runtime_builder::EzkvmRuntimeBuilder;
pub use schema::{EZKVM_CONFIG_SCHEMA_VERSION, EzkvmConfigSchema, HostSchema, Metadata};
pub use schema_builder::EzkvmSchemaBuilder;
pub use validation::ConformanceError;
pub use virtual_machine::{Bios, Boot, Device, Machine, VirtualMachine};

pub type EzkvmImportArgs = EzkvmInputArgs;
pub type EzkvmExportArgs = EzkvmOutputArgs;

/// Imports ezkvm YAML into RuntimeConfig.
pub struct EzkvmImporter;

/// Exports RuntimeConfig back to ezkvm YAML.
pub struct EzkvmExporter;

#[cfg(test)]
pub use {errors::Severity, validation::ValidationReport};
