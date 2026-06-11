//! ezkvm source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

mod diagnostics;
mod exporter;
mod importer;

pub use exporter::EzkvmOutputArgs;
pub use importer::EzkvmInputArgs;

/// Imports ezkvm YAML into RuntimeConfig.
pub struct EzkvmImporter;

/// Exports RuntimeConfig back to ezkvm YAML.
pub struct EzkvmExporter;
