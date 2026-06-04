use std::path::PathBuf;

use crate::config_format::{ExportError, ExportOptions, Exporter, EzkvmExporter, RuntimeConfig};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EzkvmOutputArgs {
    #[serde(rename = "output.host")]
    pub output_host: String,
    #[serde(rename = "output.vm")]
    pub output_vm: Option<String>,
}

impl Exporter for EzkvmExporter {
    fn export(&self, runtime: &RuntimeConfig, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let (_output_host, output_vm) = match args {
            ExportOptions::Ezkvm { host, vm } => (host, vm),
            _ => return Err(ExportError::InvalidFormat),
        };

        let path = output_vm
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}.yaml", runtime.metadata.vm_name)));

        let content = runtime.to_string();

        std::fs::write(&path, content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", path.display(), e)))?;

        Ok(path)
    }
}
