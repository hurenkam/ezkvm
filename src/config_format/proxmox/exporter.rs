//! Proxmox configuration exporter for writing runtime state in `.conf` form.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

use std::path::PathBuf;

use crate::config_format::proxmox::{
    ProxmoxMarshaler, ProxmoxSchemaBuilder, storage_resolver::ProxmoxStorageConfig,
};
use crate::config_format::stages::{Marshaler, SchemaBuilder};
use crate::config_format::{ExportError, ExportOptions, Exporter, ProxmoxExporter};
use crate::runtime_model::RuntimeModel;

impl Exporter for ProxmoxExporter {
    /// Writes the runtime configuration to a Proxmox VM configuration file.
    ///
    /// Maps the runtime model to a `ProxmoxConfigSchema`, renders it to `.conf` text,
    /// and writes the result to the path given by `output.vm`.
    fn export(&self, runtime: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let (output_storage, output_vm) = match args {
            ExportOptions::Proxmox { storage, vm } => (storage, vm),
            _ => return Err(ExportError::InvalidFormat),
        };

        let storage_text = std::fs::read_to_string(&output_storage)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", output_storage, e)))?;
        let storage_config = ProxmoxStorageConfig::parse(&storage_text)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {}", output_storage, e)))?;

        let schema = ProxmoxSchemaBuilder { storage_config }
            .build(runtime)
            .map_err(|e| ExportError::ExportFailed(format!("runtime mapping failed: {e}")))?;

        let content = ProxmoxMarshaler
            .marshal(&schema)
            .map_err(ExportError::ExportFailed)?;
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
        ExportOptions, Exporter, ProxmoxExporter,
        proxmox::{schema::ProxmoxConfigSchema, storage_resolver::ProxmoxStorageConfig},
    };
    use crate::runtime_model::RuntimeModelBuilder;

    fn build_runtime(conf_text: &str) -> crate::runtime_model::RuntimeModel {
        let schema = ProxmoxConfigSchema::parse(conf_text).expect("config should parse");
        let storage = storage_cfg();
        RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
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
        use crate::runtime_model::RuntimeModelBuilder;

        let schema = ProxmoxConfigSchema::parse(
            "name: x\nmachine: q35\nmemory: 1024\ncpu: host\ncores: 1\nsockets: 1\n",
        )
        .unwrap();
        let storage = storage_cfg();
        let runtime = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage).unwrap();

        let result = ProxmoxExporter.export(
            runtime,
            ExportOptions::Qemu {
                vm: Some("out.cmd".to_string()),
            },
        );

        assert!(matches!(result, Err(ExportError::InvalidFormat)));
    }
}
