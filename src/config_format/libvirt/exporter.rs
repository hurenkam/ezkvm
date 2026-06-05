//! libvirt XML exporter for rendering the runtime configuration to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/linux/

use std::path::PathBuf;

use crate::config_format::{ExportError, ExportOptions, Exporter, LibvirtExporter, RuntimeConfig};

impl Exporter for LibvirtExporter {
    /// Writes the runtime configuration to a libvirt XML file.
    fn export(&self, runtime: &RuntimeConfig, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let output_vm = match args {
            ExportOptions::Libvirt { vm } => vm,
            _ => return Err(ExportError::InvalidFormat),
        };

        let path = PathBuf::from(output_vm);
        let content = runtime.to_string();

        std::fs::write(&path, content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", path.display(), e)))?;

        Ok(path)
    }
}
