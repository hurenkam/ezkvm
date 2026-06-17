//! ezkvm YAML exporter for writing the canonical runtime configuration back to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

use std::path::PathBuf;

use crate::{config_format::{ExportError, ExportOptions, Exporter, EzkvmExporter, RuntimeConfig, RuntimeModelExporter}, runtime_model::RuntimeModel};

/// Arguments required to export a runtime configuration to ezkvm YAML.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EzkvmOutputArgs {
    /// Path to the host configuration used for export context.
    #[serde(rename = "output.host")]
    pub output_host: String,
    /// Optional path to write the exported VM configuration.
    #[serde(rename = "output.vm")]
    pub output_vm: Option<String>,
}

impl Exporter for EzkvmExporter {
    /// Writes the runtime configuration as ezkvm YAML.
    fn export(&self, runtime: &RuntimeConfig, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let (_output_host, output_vm) = match args {
            ExportOptions::Ezkvm { host, vm } => (host, vm),
            _ => return Err(ExportError::InvalidFormat),
        };

        let path = output_vm
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}.yaml", runtime.metadata.vm_name)));

        let content = serde_yaml::to_string(runtime).map_err(|e| {
            ExportError::ExportFailed(format!("Failed to serialize runtime config: {}", e))
        })?;

        std::fs::write(&path, content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", path.display(), e)))?;

        Ok(path)
    }
}

impl RuntimeModelExporter for EzkvmExporter {
    /// Exports the runtime configuration as ezkvm YAML.
    fn export(&self, runtime: &RuntimeModel, _args: ExportOptions) -> Result<PathBuf, ExportError> {
        let runtime_config = RuntimeConfig::try_from(runtime)
            .map_err(|e| ExportError::ExportFailed(format!("Failed to convert runtime model: {}", e)))?;

        let path = PathBuf::from(format!("{}.yaml", runtime_config.metadata.vm_name));
        Exporter::export(self, &runtime_config, ExportOptions::Ezkvm { host: String::new(), vm: Some(path.to_string_lossy().to_string()) })
    }
}