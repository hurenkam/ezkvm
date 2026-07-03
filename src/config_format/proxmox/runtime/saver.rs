//! File I/O for Proxmox VM configurations: load and save `.conf` files.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

use std::path::PathBuf;

use crate::{
    config_format::{
        ExportError, RuntimeModelSaver, SchemaBuilder,
        proxmox::{ProxmoxOptions, ProxmoxSchemaBuilder, storage_resolver::ProxmoxStorageConfig},
    },
    runtime_model::RuntimeModel,
};

/// Exports the canonical runtime model to Proxmox-style configuration text.
#[allow(dead_code)]
pub struct ProxmoxSaver;

impl RuntimeModelSaver for ProxmoxSaver {
    type Args = ProxmoxOptions;
    type Error = ExportError;

    fn save(&self, model: RuntimeModel, args: Self::Args) -> Result<(), Self::Error> {
        let storage_path = args.storage;
        let output_path = args.file;

        let storage_text = std::fs::read_to_string(&storage_path)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", storage_path, e)))?;
        let storage_config = ProxmoxStorageConfig::parse(&storage_text)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", storage_path, e)))?;

        let schema = ProxmoxSchemaBuilder::default()
            .with_storage_config(storage_config)
            .with_runtime(model)
            .build()
            .map_err(|e| ExportError::ExportFailed(format!("runtime mapping failed: {e}")))?;

        let content = schema.render();
        let path = PathBuf::from(&output_path);

        std::fs::write(&path, &content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {e}", path.display())))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config_format::{
        ProxmoxOptions, RuntimeBuilder, RuntimeModelSaver,
        proxmox::{
            ProxmoxRuntimeBuilder, runtime::saver::ProxmoxSaver, schema::ProxmoxConfigSchema,
            storage_resolver::ProxmoxStorageConfig,
        },
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
        let result = ProxmoxSaver.save(
            runtime,
            ProxmoxOptions {
                storage: storage_path,
                file: path_str.clone(),
            },
        );

        assert!(result.is_ok(), "export should succeed: {:?}", result);

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
        ProxmoxSaver
            .save(
                runtime,
                ProxmoxOptions {
                    storage: storage_path,
                    file: path_str.clone(),
                },
            )
            .expect("export should succeed");

        let written = std::fs::read_to_string(&path_str).expect("output file should exist");
        ProxmoxConfigSchema::parse(&written)
            .expect("exported file content should parse as valid Proxmox conf");
    }

    #[test]
    fn export_rejects_non_proxmox_options() {
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

        let result = ProxmoxSaver.save(
            runtime,
            ProxmoxOptions {
                storage: storage_cfg_path("invalid"),
                file: output_path("invalid"),
            },
        );

        assert!(result.is_err(), "export should fail: {:?}", result);
    }
}
