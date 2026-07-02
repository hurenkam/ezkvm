//! File I/O for QEMU command files: import and export.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

use std::path::PathBuf;

use crate::{
    config_format::{
        ExportError, ExportOptions, Exporter, ImportError, ImportOptions, Importer,
        qemu_cmd::{QemuMarshaler, QemuRuntimeBuilder, QemuSchemaBuilder, parser::QemuParser},
        stages::{Marshaler, Parser, RuntimeBuilder, SchemaBuilder},
    },
    runtime_model::RuntimeModel,
};

// ---------------------------------------------------------------------------
// Importer
// ---------------------------------------------------------------------------

impl Importer for crate::config_format::QemuImporter {
    /// Imports a QEMU command-file into the canonical runtime model.
    fn import(&self, args: ImportOptions) -> Result<RuntimeModel, ImportError> {
        let source_path = match args {
            ImportOptions::Qemu { vm } => vm,
            _ => return Err(ImportError::InvalidFormat),
        };

        let source_text = std::fs::read_to_string(&source_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", source_path, e)))?;

        let schema = QemuParser
            .parse(&source_text)
            .map_err(ImportError::ImportFailed)?;

        QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .map_err(ImportError::ImportFailed)
    }
}

// ---------------------------------------------------------------------------
// Exporter
// ---------------------------------------------------------------------------

impl Exporter for crate::config_format::QemuExporter {
    fn export(&self, model: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let output_vm = match args {
            ExportOptions::Qemu { vm } => vm,
            _ => return Err(ExportError::InvalidFormat),
        };

        let schema = QemuSchemaBuilder::default()
            .with_runtime(model)
            .build()
            .map_err(|e| ExportError::ExportFailed(format!("runtime mapping failed: {e}")))?;

        let content = QemuMarshaler
            .marshal(&schema)
            .map_err(ExportError::ExportFailed)?;

        let path = output_vm
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runtime.qemu.cmd"));

        std::fs::write(&path, content)
            .map_err(|e| ExportError::ExportFailed(format!("{}: {e}", path.display())))?;

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        config_format::{ExportOptions, Exporter, ImportOptions, Importer, qemu_cmd::QemuExporter},
        runtime_model::{
            BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, Memory, Q35Chipset,
            RuntimeModel, SeaBiosModel,
        },
    };

    fn build_runtime(name: &str) -> RuntimeModel {
        let mut busses = BusRegister::new();
        let chipset = Chipset::Q35(Q35Chipset::new(&mut busses));
        RuntimeModel::new(
            name.to_string(),
            Cpu::new(CpuModel::Host, 2, 1, 1),
            Memory::megabytes(2048),
            chipset,
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            None,
            None,
            None,
            None,
            None,
            None,
            busses,
        )
    }

    #[test]
    fn exports_runtime_to_qemu_command_file() {
        let runtime = build_runtime("export-qemu-vm");

        let output = "/tmp/ezkvm-test-qemu-export.cmd";
        let path = QemuExporter
            .export(
                runtime,
                ExportOptions::Qemu {
                    vm: Some(output.to_string()),
                },
            )
            .expect("export should succeed");

        assert_eq!(path, PathBuf::from(output));

        let content = std::fs::read_to_string(output).expect("output should be readable");
        assert!(content.contains("qemu-system-x86_64"));
        assert!(content.contains("-name export-qemu-vm"));
    }

    #[test]
    fn exports_identity_fields_to_qemu_command_file() {
        let mut busses = BusRegister::new();
        let runtime = RuntimeModel::new(
            "identity-vm".to_string(),
            Cpu::new(CpuModel::Host, 2, 1, 1),
            Memory::megabytes(2048),
            Chipset::Q35(Q35Chipset::new(&mut busses)),
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string()),
            Some("11111111-2222-3333-4444-555555555555".to_string()),
            None,
            None,
            None,
            None,
            busses,
        );

        let output = "/tmp/ezkvm-test-qemu-export-identity.cmd";
        let _ = QemuExporter
            .export(
                runtime,
                ExportOptions::Qemu {
                    vm: Some(output.to_string()),
                },
            )
            .expect("export should succeed");

        let content = std::fs::read_to_string(output).expect("output should be readable");
        assert!(content.contains("-smbios type=1,uuid=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"));
        assert!(content.contains("-device vmgenid,guid=11111111-2222-3333-4444-555555555555"));
    }

    #[test]
    fn imports_basic_qemu_command_file() {
        let path = "/tmp/ezkvm-test-qemu-import-basic.cmd";
        std::fs::write(
            path,
            "qemu-system-x86_64 -name vm1 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1",
        )
        .expect("test qemu command should be written");

        let runtime = crate::config_format::QemuImporter
            .import(ImportOptions::Qemu {
                vm: path.to_string(),
            })
            .expect("import should succeed");

        assert_eq!(runtime.name(), "vm1");
        assert_eq!(
            runtime.memory().qemu_args(runtime.cpu()),
            vec!["-m", "2048M"]
        );
    }
}
