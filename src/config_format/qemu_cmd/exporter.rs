//! QEMU command-file exporter for rendering the runtime configuration to disk.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

use std::path::PathBuf;

use crate::{
    config_format::{
        ExportError, ExportOptions, Exporter, QemuExporter,
        qemu_cmd::{QemuMarshaler, QemuSchemaBuilder},
        stages::{Marshaler, SchemaBuilder},
    },
    runtime_model::RuntimeModel,
};

impl Exporter for QemuExporter {
    fn export(&self, model: RuntimeModel, args: ExportOptions) -> Result<PathBuf, ExportError> {
        let output_vm = match args {
            ExportOptions::Qemu { vm } => vm,
            _ => return Err(ExportError::InvalidFormat),
        };

        let schema = QemuSchemaBuilder
            .build(model)
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
        config_format::{ExportOptions, Exporter, QemuExporter},
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
}
