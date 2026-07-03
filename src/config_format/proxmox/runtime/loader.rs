//! File I/O for Proxmox VM configurations: load and save `.conf` files.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

use crate::{
    config_format::{
        ImportError, RuntimeBuilder, RuntimeModelLoader,
        proxmox::{
            ProxmoxConfigSchema, ProxmoxOptions, ProxmoxRuntimeBuilder,
            storage_resolver::ProxmoxStorageConfig,
        },
    },
    runtime_model::RuntimeModel,
};

/// Imports Proxmox VM configuration files into the canonical runtime model.
#[allow(dead_code)]
pub struct ProxmoxLoader;

impl RuntimeModelLoader for ProxmoxLoader {
    type Args = ProxmoxOptions;
    type Error = ImportError;

    fn load(&self, args: Self::Args) -> Result<RuntimeModel, Self::Error> {
        let storage_path = args.storage;
        let source_path = args.file;

        let storage_text = std::fs::read_to_string(&storage_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", storage_path, e)))?;
        let storage_config = ProxmoxStorageConfig::parse(&storage_text)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", storage_path, e)))?;

        let source_text = std::fs::read_to_string(&source_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", source_path, e)))?;

        let schema = ProxmoxConfigSchema::parse(&source_text)
            .map_err(|e| ImportError::ImportFailed(e.to_string()))?;

        ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_config)
            .with_schema(schema)
            .build()
            .map_err(|e| ImportError::ImportFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::config_format::{
        ProxmoxOptions, RuntimeModelLoader, proxmox::runtime::loader::ProxmoxLoader,
    };

    fn storage_cfg_text() -> &'static str {
        r#"
dir: local
    path /var/lib/vz

lvmthin: vm-pool
    vgname vm

lvmthin: vm1-pool
    vgname vm1
"#
    }

    #[test]
    fn imports_basic_proxmox_config_file() {
        let path = "/tmp/ezkvm-test-proxmox-import-basic.conf";
        let storage_path = "/tmp/ezkvm-test-proxmox-import-storage.cfg";
        std::fs::write(
            path,
            "name: import-vm\nmachine: q35\nmemory: 2048\ncpu: host\ncores: 2\nsockets: 1\n",
        )
        .expect("test conf should be written");
        std::fs::write(storage_path, storage_cfg_text()).expect("storage cfg should be written");

        let runtime = ProxmoxLoader
            .load(ProxmoxOptions {
                storage: storage_path.to_string(),
                file: path.to_string(),
            })
            .expect("load should succeed");

        assert_eq!(runtime.name(), "import-vm");
    }
}
