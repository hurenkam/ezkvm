//! Structured importer and exporter options for the supported configuration formats.
//!
//! Related documentation:
//! - src/README.md
//! - src/config_format/mod.rs

use std::path::PathBuf;

use super::{
    Exporter, EzkvmExporter, EzkvmImporter, Importer, LibvirtExporter, LibvirtImporter,
    ProxmoxExporter, ProxmoxImporter, QemuExporter, QemuImporter, RuntimeConfig,
};

/// Options for selecting and configuring a configuration importer.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ImportOptions {
    /// Imports an ezkvm YAML configuration using host and VM paths.
    Ezkvm { host: String, vm: String },
    /// Imports a Proxmox VM configuration together with the storage layout.
    Proxmox { storage: String, vm: String },
    /// Imports a QEMU command file.
    Qemu { vm: String },
    /// Imports a libvirt XML file.
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
    /// Exports the runtime model as ezkvm YAML.
    Ezkvm { host: String, vm: Option<String> },
    /// Exports the runtime model as a Proxmox VM configuration.
    Proxmox { storage: String, vm: String },
    /// Exports the runtime model as a QEMU command file.
    Qemu { vm: Option<String> },
    /// Exports the runtime model as libvirt XML.
    Libvirt { vm: String },
}

impl ExportOptions {
    /// Dispatches the options to the matching exporter and writes the runtime model.
    ///
    /// # Returns
    ///
    /// The path written by the exporter, or a stringified error message when export fails.
    pub fn export_runtime(&self, runtime: &RuntimeConfig) -> Result<PathBuf, String> {
        match self {
            ExportOptions::Ezkvm { .. } => EzkvmExporter.export(runtime, self.clone()),
            ExportOptions::Proxmox { .. } => ProxmoxExporter.export(runtime, self.clone()),
            ExportOptions::Qemu { .. } => QemuExporter.export(runtime, self.clone()),
            ExportOptions::Libvirt { .. } => LibvirtExporter.export(runtime, self.clone()),
        }
        .map_err(|error| format!("export failed: {error}"))
    }
}
