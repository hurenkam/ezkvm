//! Proxmox source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

mod runtime;
mod schema;
mod storage_resolver;

pub use runtime::ProxmoxRuntimeBuilder;
pub use schema::ProxmoxConfigSchema;
pub use schema::ProxmoxSchemaBuilder;
pub use storage_resolver::ProxmoxStorageConfig;
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxmoxOptions {
    pub storage: String,
    pub file: String,
}
