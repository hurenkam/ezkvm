//! QEMU command-file exporter for rendering the runtime configuration to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

use std::path::PathBuf;

use crate::{
    config_format::{ExportError, ExportOptions, Exporter, QemuExporter},
    runtime_model::RuntimeModel,
};
impl Exporter for QemuExporter {
    fn export(&self, _model: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let output_vm = match args {
            ExportOptions::Qemu { vm } => vm,
            _ => return Err(ExportError::InvalidFormat),
        };

        let _path = output_vm
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runtime.qemu.cmd"));

        Err(ExportError::UnsupportedExporter(
            "QEMU command file exporter is not implemented yet".to_string(),
        ))
    }
}
