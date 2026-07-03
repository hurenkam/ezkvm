//! SchemaBuilder stage: `RuntimeModel` → `QemuCommandSchema`.

use crate::{
    config_format::{
        SchemaBuilder,
        qemu_cmd::{
            parser::parse_known_fields,
            schema::{QemuCommandSchema, QemuKnownFields},
        },
    },
    runtime_model::RuntimeModel,
};

/// Builds qemu command schema from runtime model.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Default)]
pub struct QemuSchemaBuilder {
    runtime: Option<RuntimeModel>,
}

impl SchemaBuilder for QemuSchemaBuilder {
    type Schema = QemuCommandSchema;

    fn with_runtime(self, runtime: RuntimeModel) -> Self {
        Self {
            runtime: Some(runtime),
        }
    }

    fn build(self) -> Result<QemuCommandSchema, String> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            "QemuSchemaBuilder requires a runtime model to build schema".to_string()
        })?;
        let qemu_command = runtime.qemu_command();
        let executable = qemu_command
            .first()
            .cloned()
            .ok_or_else(|| "runtime model produced an empty qemu command".to_string())?;
        let args = qemu_command.into_iter().skip(1).collect::<Vec<_>>();

        let mut known: QemuKnownFields = parse_known_fields(&args);
        known.name = Some(runtime.name().clone());

        Ok(QemuCommandSchema::new(executable, args, known))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_model::{
        BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, Memory, Q35Chipset,
        RuntimeModel, SeaBiosModel,
    };

    #[test]
    fn builds_schema_from_runtime_canonical_command() {
        let mut busses = BusRegister::new();
        let runtime = RuntimeModel::new(
            "demo".to_string(),
            Cpu::new(CpuModel::Host, 2, 1, 1),
            Memory::megabytes(2048),
            Chipset::Q35(Q35Chipset::new(&mut busses)),
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            None,
            None,
            None,
            None,
            None,
            None,
            busses,
        );

        let schema = QemuSchemaBuilder::default()
            .with_runtime(runtime)
            .build()
            .expect("build should succeed");

        assert_eq!(schema.executable, "qemu-system-x86_64");
        assert!(schema.args.contains(&"-readconfig".to_string()));
        assert!(schema.args.contains(&"-nodefaults".to_string()));
    }
}
