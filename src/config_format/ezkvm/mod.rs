//! ezkvm source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

mod builder;
mod diagnostics;
mod exporter;
mod importer;
mod parsing;
mod renderer;
mod schema;
mod validation;
mod virtual_machine;

pub use exporter::EzkvmOutputArgs;
pub use importer::EzkvmInputArgs;
pub use parsing::{ParseError, ValidationIssue};
pub use schema::{EZKVM_CONFIG_SCHEMA_VERSION, EzkvmConfigSchema, Metadata};
pub use validation::ConformanceError;
pub use virtual_machine::{Bios, Boot, Device, Machine, VirtualMachine};

/// Imports ezkvm YAML into RuntimeConfig.
pub struct EzkvmImporter;

/// Exports RuntimeConfig back to ezkvm YAML.
pub struct EzkvmExporter;

#[cfg(test)]
pub use {parsing::Severity, validation::ValidationReport};
