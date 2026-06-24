//! Proxmox VM configuration importer for translating `.conf` files into the runtime model.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

use crate::{
    config_format::{
        ImportError, ImportOptions, Importer, ProxmoxImporter,
        proxmox::{ProxmoxParser, ProxmoxRuntimeBuilder, storage_resolver::ProxmoxStorageConfig},
        stages::{Parser, RuntimeBuilder},
    },
    runtime_model::RuntimeModel,
};

impl Importer for ProxmoxImporter {
    /// Imports a Proxmox VM configuration into the canonical runtime model.
    fn import(&self, args: ImportOptions) -> Result<RuntimeModel, ImportError> {
        let (storage_path, source_path) = match args {
            ImportOptions::Proxmox { storage, vm } => (storage, vm),
            _ => return Err(ImportError::InvalidFormat),
        };

        let storage_text = std::fs::read_to_string(&storage_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", storage_path, e)))?;
        let storage_config = ProxmoxStorageConfig::parse(&storage_text)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", storage_path, e)))?;

        let source_text = std::fs::read_to_string(&source_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", source_path, e)))?;

        let schema = ProxmoxParser
            .parse(&source_text)
            .map_err(|e| ImportError::ImportFailed(e.to_string()))?;

        ProxmoxRuntimeBuilder { storage_config }
            .build(schema)
            .map_err(|e| ImportError::ImportFailed(e.to_string()))
    }
}
