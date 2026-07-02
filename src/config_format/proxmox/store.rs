//! File I/O for Proxmox VM configurations: load and save `.conf` files.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

use std::path::PathBuf;

use crate::{
    config_format::{
        ExportError, ExportOptions, Exporter, ImportError, ImportOptions, Importer,
        RuntimeModelLoader,
        proxmox::{
            ProxmoxOptions, ProxmoxRuntimeBuilder, ProxmoxSchemaBuilder,
            schema::ProxmoxConfigSchema, storage_resolver::ProxmoxStorageConfig,
        },
        stages::{RuntimeBuilder, SchemaBuilder},
    },
    runtime_model::RuntimeModel,
};

// ---------------------------------------------------------------------------
// Importer
// ---------------------------------------------------------------------------

impl Importer for crate::config_format::ProxmoxImporter {
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

        let schema = ProxmoxConfigSchema::parse(&source_text)
            .map_err(|e| ImportError::ImportFailed(e.to_string()))?;

        ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_config)
            .with_schema(schema)
            .build()
            .map_err(|e| ImportError::ImportFailed(e.to_string()))
    }
}

impl RuntimeModelLoader for crate::config_format::ProxmoxImporter {
    type Args = ProxmoxOptions;
    type Error = ImportError;

    fn load(&self, args: Self::Args) -> Result<RuntimeModel, Self::Error> {
        self.import(ImportOptions::Proxmox {
            storage: args.storage,
            vm: args.file,
        })
    }
}

// ---------------------------------------------------------------------------
// Exporter
// ---------------------------------------------------------------------------

impl Exporter for crate::config_format::ProxmoxExporter {
    /// Writes the runtime configuration to a Proxmox VM configuration file.
    fn export(&self, runtime: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let (output_storage, output_vm) = match args {
            ExportOptions::Proxmox { storage, vm } => (storage, vm),
            _ => return Err(ExportError::InvalidFormat),
        };

        let storage_text = std::fs::read_to_string(&output_storage)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", output_storage, e)))?;
        let storage_config = ProxmoxStorageConfig::parse(&storage_text)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", output_storage, e)))?;

        let schema = ProxmoxSchemaBuilder::default()
            .with_storage_config(storage_config)
            .with_runtime(runtime)
            .build()
            .map_err(|e| ExportError::ExportFailed(format!("runtime mapping failed: {e}")))?;

        let content = schema.render();
        let path = PathBuf::from(&output_vm);

        std::fs::write(&path, &content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {e}", path.display())))?;

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config_format::{
        ExportOptions, Exporter, ImportOptions, Importer,
        proxmox::{
            ProxmoxExporter, ProxmoxImporter, ProxmoxRuntimeBuilder, schema::ProxmoxConfigSchema,
            storage_resolver::ProxmoxStorageConfig,
        },
        stages::RuntimeBuilder,
    };

    fn build_runtime(conf_text: &str) -> crate::runtime_model::RuntimeModel {
        let schema = ProxmoxConfigSchema::parse(conf_text).expect("config should parse");
        let storage = storage_cfg();
        ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage)
            .with_schema(schema)
            .build()
            .expect("runtime model should build")
    }

    fn output_path(name: &str) -> String {
        format!("/tmp/ezkvm-test-proxmox-export-{name}.conf")
    }

    fn storage_cfg_path(name: &str) -> String {
        format!("/tmp/ezkvm-test-proxmox-storage-{name}.cfg")
    }

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

    fn storage_cfg() -> ProxmoxStorageConfig {
        ProxmoxStorageConfig::parse(storage_cfg_text()).expect("storage cfg should parse")
    }

    #[test]
    fn exports_minimal_config_to_file() {
        let runtime = build_runtime(
            r#"
name: export-test-vm
machine: q35
memory: 4096
cpu: host
cores: 2
sockets: 1
"#,
        );
        let path_str = output_path("minimal");
        let storage_path = storage_cfg_path("minimal");
        std::fs::write(&storage_path, storage_cfg_text()).expect("storage cfg should be written");
        let result = ProxmoxExporter.export(
            runtime,
            ExportOptions::Proxmox {
                storage: storage_path,
                vm: path_str.clone(),
            },
        );

        assert!(result.is_ok(), "export should succeed: {:?}", result);
        assert_eq!(result.unwrap(), PathBuf::from(&path_str));

        let written = std::fs::read_to_string(&path_str).expect("output file should exist");
        assert!(written.contains("name: export-test-vm"));
        assert!(written.contains("machine: q35"));
        assert!(written.contains("memory: 4096"));
        assert!(written.contains("cpu: host"));
    }

    #[test]
    fn exported_file_is_parseable_as_proxmox_conf() {
        let runtime = build_runtime(
            r#"
name: parse-check-vm
machine: q35
memory: 8192
cpu: host
cores: 4
sockets: 1
scsi0: vm-pool:vm-100-disk-0,cache=writeback,size=32G
net0: virtio=DE:AD:BE:EF:00:42,bridge=vmbr0
"#,
        );
        let path_str = output_path("parseable");
        let storage_path = storage_cfg_path("parseable");
        std::fs::write(&storage_path, storage_cfg_text()).expect("storage cfg should be written");
        ProxmoxExporter
            .export(
                runtime,
                ExportOptions::Proxmox {
                    storage: storage_path,
                    vm: path_str.clone(),
                },
            )
            .expect("export should succeed");

        let written = std::fs::read_to_string(&path_str).expect("output file should exist");
        ProxmoxConfigSchema::parse(&written)
            .expect("exported file content should parse as valid Proxmox conf");
    }

    #[test]
    fn export_rejects_non_proxmox_options() {
        use crate::config_format::ExportError;

        let schema = ProxmoxConfigSchema::parse(
            "name: x\nmachine: q35\nmemory: 1024\ncpu: host\ncores: 1\nsockets: 1\n",
        )
        .unwrap();
        let storage = storage_cfg();
        let runtime = ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage)
            .with_schema(schema)
            .build()
            .unwrap();

        let result = ProxmoxExporter.export(
            runtime,
            ExportOptions::Qemu {
                vm: Some("out.cmd".to_string()),
            },
        );

        assert!(matches!(result, Err(ExportError::InvalidFormat)));
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

        let runtime = ProxmoxImporter
            .import(ImportOptions::Proxmox {
                storage: storage_path.to_string(),
                vm: path.to_string(),
            })
            .expect("import should succeed");

        assert_eq!(runtime.name(), "import-vm");
    }
}
