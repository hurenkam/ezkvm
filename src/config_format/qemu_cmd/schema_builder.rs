//! SchemaBuilder stage: `RuntimeModel` -> `QemuCommandSchema`.

use crate::{
    config_format::{
        qemu_cmd::{
            parser::parse_known_fields,
            schema::{QemuCommandSchema, QemuKnownFields},
        },
        stages::SchemaBuilder,
    },
    runtime_model::RuntimeModel,
};

/// Builds qemu command schema from runtime model.
pub struct QemuSchemaBuilder;

impl SchemaBuilder for QemuSchemaBuilder {
    type Schema = QemuCommandSchema;

    fn build(&self, runtime: RuntimeModel) -> Result<QemuCommandSchema, String> {
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
    use crate::{
        config_format::{qemu_cmd::schema_builder::QemuSchemaBuilder, stages::SchemaBuilder},
        runtime_model::{
            BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, Memory, Q35Chipset,
            RuntimeModel, SeaBiosModel,
        },
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

        let schema = QemuSchemaBuilder
            .build(runtime)
            .expect("build should succeed");

        assert_eq!(schema.executable, "qemu-system-x86_64");
        assert!(schema.args.contains(&"-readconfig".to_string()));
        assert!(schema.args.contains(&"-nodefaults".to_string()));
    }
}
