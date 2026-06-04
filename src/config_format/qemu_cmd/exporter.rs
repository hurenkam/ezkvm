use std::path::PathBuf;

use crate::config_format::{ExportError, ExportOptions, Exporter, QemuExporter, RuntimeConfig};

impl Exporter for QemuExporter {
    fn export(&self, runtime: &RuntimeConfig, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let output_vm = match args {
            ExportOptions::Qemu { vm } => vm,
            _ => return Err(ExportError::InvalidFormat),
        };

        let path = output_vm
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}.qemu.cmd", runtime.metadata.vm_name)));

        let content = runtime.to_string();

        std::fs::write(&path, content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", path.display(), e)))?;

        Ok(path)
    }
}
