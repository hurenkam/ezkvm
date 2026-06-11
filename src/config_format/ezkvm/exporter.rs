//! ezkvm YAML exporter for writing the canonical runtime configuration back to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

use std::path::PathBuf;

use crate::config_format::{ExportError, ExportOptions, Exporter, EzkvmExporter, RuntimeConfig};

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
