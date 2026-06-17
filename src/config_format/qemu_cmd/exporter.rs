//! QEMU command-file exporter for rendering the runtime configuration to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

use std::path::PathBuf;

use crate::{
    config_format::{ExportError, ExportOptions, QemuExporter, RuntimeModelExporter},
    runtime_model::RuntimeModel,
};
/*
impl Exporter for QemuExporter {
    /// Writes the runtime configuration to a QEMU command file.
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
*/
impl RuntimeModelExporter for QemuExporter {
    fn export(&self, _model: &RuntimeModel, _args: ExportOptions) -> Result<PathBuf, ExportError> {
        /*
                let output_vm = match args {
                    ExportOptions::Qemu { vm } => vm,
                    _ => return Err(ExportError::InvalidFormat),
                };

                let path = output_vm
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(format!("{}.qemu.cmd", model.name())));

                let content = model.to_string();

                std::fs::write(&path, content)
                    .map_err(|e| ExportError::ExportFailed(format!("{}: {}", path.display(), e)))?;

                Ok(path)
        */
        Err(ExportError::UnsupportedExporter(
            "QEMU command file exporter is not implemented yet".to_string(),
        ))
    }
}
