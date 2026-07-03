use std::path::PathBuf;

use crate::{
    config_format::{
        ExportError, Marshaler, RuntimeModelSaver, SchemaBuilder,
        qemu_cmd::{QemuSchemaBuilder, marshaler::QemuMarshaler},
    },
    runtime_model::RuntimeModel,
};

/// Arguments required to export a QEMU command file.
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuOutputArgs {
    /// Optional destination path for the generated command file.
    #[serde(rename = "output.vm")]
    pub output_vm: Option<String>,
}

/// Exports the canonical runtime model to a QEMU command file.
#[allow(dead_code)] // TODO: wire to CLI
pub struct QemuSaver;

impl RuntimeModelSaver for QemuSaver {
    type Args = QemuOutputArgs;
    type Error = ExportError;

    fn save(&self, model: RuntimeModel, args: Self::Args) -> Result<(), Self::Error> {
        let output_vm = args.output_vm;

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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config_format::{
            RuntimeModelSaver,
            qemu_cmd::saver::{QemuOutputArgs, QemuSaver},
        },
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
        let result = QemuSaver.save(
            runtime,
            QemuOutputArgs {
                output_vm: Some(output.to_string()),
            },
        );

        assert!(result.is_ok(), "export should succeed");

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
        let _ = QemuSaver
            .save(
                runtime,
                QemuOutputArgs {
                    output_vm: Some(output.to_string()),
                },
            )
            .expect("export should succeed");

        let content = std::fs::read_to_string(output).expect("output should be readable");
        assert!(content.contains("-smbios type=1,uuid=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"));
        assert!(content.contains("-device vmgenid,guid=11111111-2222-3333-4444-555555555555"));
    }
}
