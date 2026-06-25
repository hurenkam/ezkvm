//! RuntimeBuilder stage: `QemuCommandSchema` -> `RuntimeModel`.

use std::sync::Arc;

use crate::{
    config_format::{qemu_cmd::schema::QemuCommandSchema, stages::RuntimeBuilder},
    runtime_model::{
        BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, Display, DisplayModelBuilder,
        GuestAgent, GuestAgentModelBuilder, I440fxChipset, Memory, PciDeviceApi, PciDeviceType,
        PcieAddress, PcieDeviceApi, PcieDeviceType, Q35Chipset, RuntimeModel, SeaBiosModel,
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

        let display = parse_display(&schema.args);
        let guest_agent = parse_guest_agent(&schema.args);
        let smbios_uuid = parse_smbios_uuid(&schema.args);
        let vmgenid = parse_vmgenid(&schema.args);
        register_gpu_from_args(&schema.args, &bus_register)?;

        Ok(RuntimeModel::new(
            name,
            cpu,
            memory,
            chipset,
            boot,
            smbios_uuid,
            vmgenid,
            None,
            display.map(DisplayModelBuilder::build),
            None,
            guest_agent.map(GuestAgentModelBuilder::build),
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

fn parse_display(args: &[String]) -> Option<Display> {
    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-spice"
        {
            let mut listen = "0.0.0.0".to_string();
            let mut port = None;
            let mut disable_ticketing = false;

            for token in value.split(',').map(str::trim) {
                if let Some(v) = token.strip_prefix("addr=") {
                    listen = v.to_string();
                }
                if let Some(v) = token.strip_prefix("port=") {
                    port = v.parse::<u16>().ok();
                }
                if let Some(v) = token.strip_prefix("tls-port=") {
                    port = v.parse::<u16>().ok();
                }
                if token == "disable-ticketing=on" {
                    disable_ticketing = true;
                }
            }

            if let Some(port) = port {
                return serde_json::from_value(serde_json::json!({
                    "spice": {
                        "listen": listen,
                        "port": port,
                        "disable_ticketing": disable_ticketing
                    }
                }))
                .ok();
            }
        }
    }

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-vnc"
            && value != "none"
            && !value.starts_with("unix:")
        {
            let target = value.split(',').next().unwrap_or_default();
            if let Some((listen, port)) = target.rsplit_once(':')
                && let Ok(port) = port.parse::<u16>()
            {
                return serde_json::from_value(serde_json::json!({
                    "vnc": {
                        "listen": listen,
                        "port": port
                    }
                }))
                .ok();
            }
        }
    }

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-display"
        {
            if value.starts_with("gtk") {
                return serde_json::from_value(serde_json::json!({ "gtk": {} })).ok();
            }
            if value.starts_with("sdl") {
                return serde_json::from_value(serde_json::json!({ "sdl": {} })).ok();
            }
            if value == "none"
                && args
                    .windows(2)
                    .any(|w| matches!(w, [a, b] if a == "-vnc" && b == "none"))
            {
                return serde_json::from_value(serde_json::json!({ "looking_glass": {} })).ok();
            }
        }
    }

    None
}

fn parse_guest_agent(args: &[String]) -> Option<GuestAgent> {
    if args
        .iter()
        .any(|arg| arg.contains("org.qemu.guest_agent.0"))
    {
        return Some(GuestAgent { enabled: true });
    }
    None
}

fn parse_smbios_uuid(args: &[String]) -> Option<String> {
    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-smbios"
            && value.contains("type=1")
        {
            for token in value.split(',').map(str::trim) {
                if let Some(uuid) = token.strip_prefix("uuid=") {
                    let uuid = uuid.trim();
                    if !uuid.is_empty() {
                        return Some(uuid.to_string());
                    }
                }
            }
        }
    }

    None
}

fn parse_vmgenid(args: &[String]) -> Option<String> {
    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-device"
            && value.starts_with("vmgenid")
        {
            for token in value.split(',').map(str::trim) {
                if let Some(guid) = token.strip_prefix("guid=") {
                    let guid = guid.trim();
                    if !guid.is_empty() {
                        return Some(guid.to_string());
                    }
                }
            }
        }
    }

    None
}

fn register_gpu_from_args(args: &[String], busses: &BusRegister) -> Result<(), String> {
    let gpu = detect_gpu(args);
    match gpu {
        Some(ImportedGpu::Standard) => match busses.pcie_busses().get(&0) {
            Some(root) => {
                let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::StandardGpu).into();
                root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))
            }
            None => Err("PCIe root bus with id 0 does not exist".to_string()),
        },
        Some(ImportedGpu::Virtio) => match busses.pcie_busses().get(&0) {
            Some(root) => {
                let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::VirtioGpu).into();
                root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))
            }
            None => Err("PCIe root bus with id 0 does not exist".to_string()),
        },
        Some(ImportedGpu::Qxl) => {
            if let Some(root) = busses.pci_busses().get(&0) {
                let device: Arc<dyn PciDeviceApi> = (&PciDeviceType::QxlGpu).into();
                root.register_pci_device(device, None)
            } else if let Some(root) = busses.pcie_busses().get(&0) {
                let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::StandardGpu).into();
                root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))
            } else {
                Err("Neither PCI nor PCIe root bus exists".to_string())
            }
        }
        Some(ImportedGpu::Headless) | None => Ok(()),
    }
}

#[derive(Clone, Copy)]
enum ImportedGpu {
    Standard,
    Virtio,
    Qxl,
    Headless,
}

fn detect_gpu(args: &[String]) -> Option<ImportedGpu> {
    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-vga"
        {
            let token = value
                .split(',')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if token == "none" {
                return Some(ImportedGpu::Headless);
            }
            if token.starts_with("qxl") {
                return Some(ImportedGpu::Qxl);
            }
            if token.starts_with("virtio") {
                return Some(ImportedGpu::Virtio);
            }
            return Some(ImportedGpu::Standard);
        }
    }

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-device"
        {
            let token = value
                .split(',')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if token == "vga" || token == "std-vga" {
                return Some(ImportedGpu::Standard);
            }
            if token.contains("virtio-vga") || token.contains("virtio-gpu") {
                return Some(ImportedGpu::Virtio);
            }
            if token.contains("qxl") {
                return Some(ImportedGpu::Qxl);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::config_format::{qemu_cmd::QemuParser, stages::Parser, stages::RuntimeBuilder};

    use super::QemuRuntimeBuilder;

    #[test]
    fn imports_display_guest_agent_and_gpu_from_args() {
        let cmd = "qemu-system-x86_64 -name vm1 -m 4096 -cpu host -smp 4,sockets=1,cores=4,threads=1 -spice port=5905,addr=127.0.0.1,disable-ticketing=on -device virtio-vga -chardev socket,path=/tmp/vm1.qga,server=on,wait=off,id=qga0 -device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder
            .build(schema)
            .expect("runtime build should succeed");

        let rendered = runtime.qemu_command();
        assert!(rendered.iter().any(|arg| arg.contains("virtio-gpu-pci")));
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("org.qemu.guest_agent.0"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("port=5905,addr=127.0.0.1,disable-ticketing=on"))
        );
    }

    #[test]
    fn imports_qxl_gpu_from_vga_flag() {
        let cmd = "qemu-system-x86_64 -name vm2 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1 -vga qxl";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder
            .build(schema)
            .expect("runtime build should succeed");

        let rendered = runtime.qemu_command();
        assert!(rendered.iter().any(|arg| arg == "qxl" || arg == "VGA"));
    }

    #[test]
    fn imports_identity_fields_from_args() {
        let cmd = "qemu-system-x86_64 -name vm3 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1 -smbios type=1,uuid=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee -device vmgenid,guid=11111111-2222-3333-4444-555555555555";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder
            .build(schema)
            .expect("runtime build should succeed");

        assert_eq!(
            runtime.smbios_uuid().as_deref(),
            Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        );
        assert_eq!(
            runtime.vmgenid().as_deref(),
            Some("11111111-2222-3333-4444-555555555555")
        );
    }
}
