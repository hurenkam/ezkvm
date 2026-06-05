//! QEMU command-file source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

mod exporter;
mod importer;

/// Imports QEMU command files into the canonical runtime model.
pub struct QemuImporter;

/// Exports the canonical runtime model to a QEMU command file.
pub struct QemuExporter;

/// Arguments required to import a QEMU command file.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuInputArgs {
    /// Path to the QEMU command file.
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

/// Arguments required to export a QEMU command file.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuOutputArgs {
    /// Optional destination path for the generated command file.
    #[serde(rename = "output.vm")]
    pub output_vm: Option<String>,
}
