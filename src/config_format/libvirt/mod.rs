//! libvirt XML source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/linux/

mod exporter;
mod importer;

/// Imports libvirt XML into the canonical runtime model.
#[allow(dead_code)] // TODO: wire to CLI
pub struct LibvirtImporter;

/// Exports the canonical runtime model to libvirt XML.
#[allow(dead_code)] // TODO: wire to CLI
pub struct LibvirtExporter;

/// Arguments required to import a libvirt XML file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibvirtInputArgs {
    /// Path to the libvirt XML file.
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

/// Arguments required to export a libvirt XML file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibvirtOutputArgs {
    /// Destination path for the generated libvirt XML file.
    #[serde(rename = "output.vm")]
    pub output_vm: String,
}
