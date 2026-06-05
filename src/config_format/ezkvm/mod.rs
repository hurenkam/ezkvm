//! ezkvm source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

mod diagnostics;
mod exporter;
mod importer;

/// ezkvm exporter input arguments.
pub use exporter::EzkvmOutputArgs;
/// ezkvm importer input arguments.
pub use importer::EzkvmInputArgs;

/// Imports ezkvm YAML into the canonical runtime model.
pub struct EzkvmImporter;

/// Exports the canonical runtime model back to ezkvm YAML.
pub struct EzkvmExporter;
