//! ezkvm YAML exporter for writing the canonical runtime configuration back to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

use std::path::PathBuf;

use crate::config_format::ezkvm::builder::EzkvmHostSchema;
use crate::config_format::ezkvm::renderer::EzkvmRuntimeModelRenderer;
use crate::config_format::{ExportError, ExportOptions, Exporter, EzkvmExporter};
use crate::runtime_model::RuntimeModel;

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
    /// Writes the runtime model as ezkvm YAML.
    fn export(&self, runtime: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let (output_host, output_vm) = match args {
            ExportOptions::Ezkvm { host, vm } => (host, vm),
            _ => return Err(ExportError::InvalidFormat),
        };

        let config_schema = EzkvmRuntimeModelRenderer::new()
            .with_runtime_model(runtime)
            .with_host_schema(EzkvmHostSchema::new(output_host))
            .render()
            .map_err(|e| {
                ExportError::ExportFailed(format!("Failed to convert to schema: {}", e))
            })?;

        let path = output_vm
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}.yaml", config_schema.metadata.vm_name)));

        let content = serde_yaml::to_string(&config_schema)
            .map_err(|e| ExportError::ExportFailed(format!("Failed to serialize schema: {}", e)))?;

        std::fs::write(&path, content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", path.display(), e)))?;

        Ok(path)
    }
}
