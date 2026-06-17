//! Structured importer and exporter options for the supported configuration formats.
//!
//! Related documentation:
//! - src/README.md
//! - src/config_format/mod.rs

use std::path::PathBuf;

use crate::{config_format::RuntimeModelExporter, runtime_model::RuntimeModel};

use super::{
    Exporter, EzkvmExporter, EzkvmImporter, Importer, LibvirtExporter, LibvirtImporter,
    ProxmoxExporter, ProxmoxImporter, QemuExporter, QemuImporter, RuntimeConfig,
};

/// Options for selecting and configuring a configuration importer.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ImportOptions {
    Ezkvm { host: String, vm: String },
    Proxmox { storage: String, vm: String },
    Qemu { vm: String },
    Libvirt { vm: String },
}

impl ImportOptions {
    /// Dispatches the options to the matching importer and returns the canonical runtime model.
    ///
    /// # Returns
    ///
    /// A validated runtime configuration, or a stringified error message when import fails.
    pub fn import_runtime(&self) -> Result<RuntimeConfig, String> {
        match self {
            ImportOptions::Ezkvm { .. } => EzkvmImporter.import(self.clone()),
            ImportOptions::Proxmox { .. } => ProxmoxImporter.import(self.clone()),
            ImportOptions::Qemu { .. } => QemuImporter.import(self.clone()),
            ImportOptions::Libvirt { .. } => LibvirtImporter.import(self.clone()),
        }
        .map_err(|error| format!("import failed: {error}"))
    }
}

/// Options for selecting and configuring a configuration exporter.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ExportOptions {
    Ezkvm { host: String, vm: Option<String> },
    Proxmox { storage: String, vm: String },
    Qemu { vm: Option<String> },
    Libvirt { vm: String },
}

impl ExportOptions {
    /// Dispatches the options to the matching exporter and writes the runtime config.
    ///
    /// # Returns
    ///
    /// The path written by the exporter, or a stringified error message when export fails.
    pub fn export_runtime(&self, runtime: &RuntimeConfig) -> Result<PathBuf, String> {
        let model = import_runtime(runtime.metadata.vm_name.clone())?;
        match self {
            ExportOptions::Ezkvm { .. } => EzkvmExporter.export(runtime, self.clone()),
            ExportOptions::Proxmox { .. } => ProxmoxExporter.export(runtime, self.clone()),
            ExportOptions::Qemu { .. } => QemuExporter.export(&model, self.clone()),
            ExportOptions::Libvirt { .. } => LibvirtExporter.export(runtime, self.clone()),
        }
        .map_err(|error| format!("export failed: {error}"))
    }
}

fn import_runtime(name: String) -> Result<RuntimeModel, String> {
    let runtime_config = ImportOptions::Ezkvm {
        host: "./dist/etc/ezkvm/host.yaml".to_string(),
        vm: name,
    }
    .import_runtime()?;
    RuntimeModel::try_from(runtime_config)
}
