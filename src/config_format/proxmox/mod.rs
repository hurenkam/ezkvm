//! Proxmox source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

mod exporter;
mod importer;

/// Imports Proxmox VM configuration files into the canonical runtime model.
pub struct ProxmoxImporter;

/// Exports the canonical runtime model to Proxmox-style configuration text.
pub struct ProxmoxExporter;

/// Arguments required to import a Proxmox VM configuration.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxmoxInputArgs {
    /// Path to the Proxmox storage configuration used during import.
    #[serde(rename = "input.storage")]
    pub input_storage: String,
    /// Path to the Proxmox VM configuration file.
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

/// Arguments required to export a runtime configuration to Proxmox format.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxmoxOutputArgs {
    /// Path to the Proxmox storage configuration used during export.
    #[serde(rename = "output.storage")]
    pub output_storage: String,
    /// Destination path for the generated Proxmox VM configuration.
    #[serde(rename = "output.vm")]
    pub output_vm: String,
}
