//! libvirt XML exporter for rendering the runtime configuration to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/linux/

use std::path::PathBuf;

use crate::config_format::{ExportError, ExportOptions, Exporter, LibvirtExporter};
use crate::runtime_model::RuntimeModel;

impl Exporter for LibvirtExporter {
    /// Writes the runtime configuration to a libvirt XML file.
    fn export(&self, _runtime: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let output_vm = match args {
            ExportOptions::Libvirt { vm } => vm,
            _ => return Err(ExportError::InvalidFormat),
        };

        let path = PathBuf::from(output_vm);
        Err(ExportError::UnsupportedExporter(format!(
            "libvirt export to '{}' is not implemented yet",
            path.display()
        )))
    }
}
