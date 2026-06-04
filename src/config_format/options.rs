use std::path::PathBuf;

use super::{
    Exporter, EzkvmExporter, EzkvmImporter, Importer, LibvirtExporter, LibvirtImporter,
    ProxmoxExporter, ProxmoxImporter, QemuExporter, QemuImporter, RuntimeConfig,
};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ImportOptions {
    Ezkvm { host: String, vm: String },
    Proxmox { storage: String, vm: String },
    Qemu { vm: String },
    Libvirt { vm: String },
}

impl ImportOptions {
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ExportOptions {
    Ezkvm { host: String, vm: Option<String> },
    Proxmox { storage: String, vm: String },
    Qemu { vm: Option<String> },
    Libvirt { vm: String },
}

impl ExportOptions {
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
