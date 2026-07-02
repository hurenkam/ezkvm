//! Proxmox source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

mod runtime_builder;
mod schema;
mod schema_builder;
mod storage_resolver;
mod store;

pub use runtime_builder::ProxmoxRuntimeBuilder;
#[allow(unused_imports)]
pub use schema::ProxmoxConfigSchema;
pub use schema_builder::ProxmoxSchemaBuilder;
pub use storage_resolver::ProxmoxStorageConfig;

/// Imports Proxmox VM configuration files into the canonical runtime model.
#[allow(dead_code)] // TODO: wire to CLI
pub struct ProxmoxImporter;

/// Exports the canonical runtime model to Proxmox-style configuration text.
#[allow(dead_code)] // TODO: wire to CLI
pub struct ProxmoxExporter;

/// Arguments required to import a Proxmox VM configuration.
#[allow(dead_code)] // TODO: wire to CLI
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
#[allow(dead_code)] // TODO: wire to CLI
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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxmoxOptions {
    pub storage: String,
    pub file: String,
}
