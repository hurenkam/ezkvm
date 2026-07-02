//! QEMU command-file source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

mod marshaler;
mod parser;
mod runtime_builder;
mod schema;
mod schema_builder;
mod store;

pub use marshaler::QemuMarshaler;
#[allow(unused_imports)]
pub use parser::QemuParser;
pub use runtime_builder::QemuRuntimeBuilder;
pub use schema_builder::QemuSchemaBuilder;

/// Imports QEMU command files into the canonical runtime model.
#[allow(dead_code)] // TODO: wire to CLI
pub struct QemuImporter;

/// Exports the canonical runtime model to a QEMU command file.
#[allow(dead_code)] // TODO: wire to CLI
pub struct QemuExporter;

/// Arguments required to import a QEMU command file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuInputArgs {
    /// Path to the QEMU command file.
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

/// Arguments required to export a QEMU command file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuOutputArgs {
    /// Optional destination path for the generated command file.
    #[serde(rename = "output.vm")]
    pub output_vm: Option<String>,
}
