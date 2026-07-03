//! libvirt XML exporter for rendering the runtime configuration to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/linux/

use std::path::PathBuf;

use crate::config_format::{ExportError, RuntimeModelSaver};
use crate::runtime_model::RuntimeModel;

/// Arguments required to export a libvirt XML file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibvirtOutputArgs {
    /// Destination path for the generated libvirt XML file.
    #[serde(rename = "output.vm")]
    pub output_vm: String,
}

/// Exports the canonical runtime model to libvirt XML.
#[allow(dead_code)] // TODO: wire to CLI
pub struct LibvirtSaver;

impl RuntimeModelSaver for LibvirtSaver {
    type Args = LibvirtOutputArgs;
    type Error = ExportError;

    fn save(&self, _model: RuntimeModel, args: Self::Args) -> Result<(), Self::Error> {
        let output_vm = args.output_vm;
        let path = PathBuf::from(output_vm);
        Err(ExportError::UnsupportedExporter(format!(
            "libvirt export to '{}' is not implemented yet",
            path.display()
        )))
    }
}
