//! RuntimeBuilder stage: `QemuCommandSchema` -> `RuntimeModel`.

use crate::{
    config_format::{qemu_cmd::schema::QemuCommandSchema, stages::RuntimeBuilder},
    runtime_model::{
        BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, I440fxChipset, Memory,
        Q35Chipset, RuntimeModel, SeaBiosModel,
    },
};

/// Builds a `RuntimeModel` from parsed qemu command schema.
pub struct QemuRuntimeBuilder;

impl RuntimeBuilder for QemuRuntimeBuilder {
    type Schema = QemuCommandSchema;

    fn build(&self, schema: QemuCommandSchema) -> Result<RuntimeModel, String> {
        let name = schema
            .known
            .name
            .clone()
            .unwrap_or_else(|| "qemu-vm".to_string());

        let cpu = Cpu::new(
            parse_cpu_model(schema.known.cpu_model.as_deref()),
            schema.known.cores.unwrap_or(1),
            schema.known.threads.unwrap_or(1),
            schema.known.sockets.unwrap_or(1),
        );

        let memory = Memory::megabytes(schema.known.memory_mb.unwrap_or(1024) as usize);

        let mut bus_register = BusRegister::new();
        let chipset = match parse_machine_chipset(schema.known.machine.as_deref()) {
            MachineChipset::Q35 => Chipset::Q35(Q35Chipset::new(&mut bus_register)),
            MachineChipset::I440fx => Chipset::I440FX(I440fxChipset::new(&bus_register)),
        };

        let boot = BootModel::new(BiosModel::SeaBios(SeaBiosModel::default()));

        Ok(RuntimeModel::new(
            name,
            cpu,
            memory,
            chipset,
            boot,
            None,
            bus_register,
        ))
    }
}

enum MachineChipset {
    Q35,
    I440fx,
}

fn parse_machine_chipset(machine: Option<&str>) -> MachineChipset {
    let Some(machine) = machine else {
        return MachineChipset::Q35;
    };

    let token = machine.trim();
    if matches!(token, "q35" | "pc-q35") || token.starts_with("pc-q35-") || token.contains("q35") {
        return MachineChipset::Q35;
    }

    if matches!(token, "i440fx" | "pc-i440fx")
        || token.starts_with("pc-i440fx-")
        || token.contains("i440fx")
    {
        return MachineChipset::I440fx;
    }

    MachineChipset::Q35
}

fn parse_cpu_model(model: Option<&str>) -> CpuModel {
    match model {
        Some("host") | None => CpuModel::Host,
        Some(_) => CpuModel::Host,
    }
}
