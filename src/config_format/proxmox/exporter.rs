use std::path::PathBuf;

use crate::config_format::{ExportError, ExportOptions, Exporter, ProxmoxExporter, RuntimeConfig};

impl Exporter for ProxmoxExporter {
    fn export(&self, runtime: &RuntimeConfig, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let (_output_storage, output_vm) = match args {
            ExportOptions::Proxmox { storage, vm } => (storage, vm),
            _ => return Err(ExportError::InvalidFormat),
        };

        let path = PathBuf::from(output_vm);
        let content = runtime.to_string();

        std::fs::write(&path, content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", path.display(), e)))?;

        Ok(path)
    }
}
