//! RuntimeBuilder stage: `QemuCommandSchema` → `RuntimeModel`.

#![allow(dead_code)] // TODO: wire to CLI
use crate::{
    config_format::{RuntimeBuilder, qemu_cmd::schema::QemuCommandSchema},
    runtime_model::{
        BalloonConfig, BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel,
        DisplayModelBuilder, EglHeadless, GuestAgentModelBuilder, I440fxChipset, LifecycleConfig,
        Memory, PowerManagementConfig, Q35Chipset, RuntimeModel, SeaBiosModel, SerialConfig, Spice,
        Vnc,
    },
};

/// Builds a `RuntimeModel` from parsed qemu command schema.
#[derive(Default)]
pub struct QemuRuntimeBuilder {
    schema: Option<QemuCommandSchema>,
}

impl RuntimeBuilder for QemuRuntimeBuilder {
    type Schema = QemuCommandSchema;

    fn with_schema(self, schema: Self::Schema) -> Self {
        Self {
            schema: Some(schema),
        }
    }

    fn build(self) -> Result<RuntimeModel, String> {
        let schema = self.schema.as_ref().ok_or_else(|| {
            "QemuRuntimeBuilder requires a schema to build runtime model".to_string()
        })?;
        let name = schema
            .known
            .name
            .clone()
            .unwrap_or_else(|| "qemu-vm".to_string());

        let (cpu_model, enabled_features, disabled_features) =
            parse_cpu_model_and_features(schema.known.cpu_model.as_deref());
        let cpu = Cpu::new(
            cpu_model,
            schema.known.cores.unwrap_or(1),
            schema.known.threads.unwrap_or(1),
            schema.known.sockets.unwrap_or(1),
        )
        .with_features(enabled_features, disabled_features);

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
        let lifecycle = parse_lifecycle(&schema.args);
        let balloon = parse_balloon(&schema.args);
        let serial = parse_serial(&schema.args);
        let iscsi_initiator = parse_iscsi_initiator(&schema.args);
        let power_management = parse_power_management(&schema.args);
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
        )
        .with_lifecycle_config(lifecycle)
        .with_balloon_config(balloon)
        .with_serial_config(serial)
        .with_iscsi_initiator(iscsi_initiator)
        .with_power_management_config(power_management))
    }
}

// ---------------------------------------------------------------------------
// Command parsing helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
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

fn parse_cpu_model_and_features(model: Option<&str>) -> (CpuModel, Vec<String>, Vec<String>) {
    let Some(spec) = model else {
        return (CpuModel::Host, Vec::new(), Vec::new());
    };

    let mut parts = spec
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let base = parts.next().unwrap_or("host");
    let model = if base.eq_ignore_ascii_case("host") {
        CpuModel::Host
    } else {
        CpuModel::Named {
            name: base.to_string(),
        }
    };

    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    for part in parts {
        if let Some(feature) = part.strip_prefix('+')
            && !feature.trim().is_empty()
        {
            enabled.push(feature.trim().to_string());
        }
        if let Some(feature) = part.strip_prefix('-')
            && !feature.trim().is_empty()
        {
            disabled.push(feature.trim().to_string());
        }
    }

    (model, enabled, disabled)
}

fn parse_iscsi_initiator(args: &[String]) -> Option<String> {
    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-iscsi"
        {
            for token in value.split(',').map(str::trim) {
                if let Some(name) = token.strip_prefix("initiator-name=") {
                    let name = name.trim();
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }

    None
}

fn parse_power_management(args: &[String]) -> Option<PowerManagementConfig> {
    let mut disable_s3 = None;
    let mut disable_s4 = None;

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-global"
        {
            if let Some(raw) = value.strip_prefix("ICH9-LPC.disable_s3=") {
                disable_s3 = Some(matches!(raw.trim(), "1" | "on" | "true"));
            }
            if let Some(raw) = value.strip_prefix("ICH9-LPC.disable_s4=") {
                disable_s4 = Some(matches!(raw.trim(), "1" | "on" | "true"));
            }
        }
    }

    if disable_s3.is_none() && disable_s4.is_none() {
        return None;
    }

    Some(PowerManagementConfig::new(disable_s3, disable_s4))
}

fn parse_display(args: &[String]) -> Option<crate::runtime_model::Display> {
    let gl_enabled = args.windows(2).any(|window| {
        if let [flag, value] = window
            && flag == "-display"
        {
            return value.starts_with("egl-headless");
        }
        false
    });

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-spice"
        {
            let mut listen = "0.0.0.0".to_string();
            let mut port = None;
            let mut tls_port = None;
            let mut tls_ciphers = None;
            let mut disable_ticketing = false;
            let mut seamless_migration = false;

            for token in value.split(',').map(str::trim) {
                if let Some(v) = token.strip_prefix("addr=") {
                    listen = v.to_string();
                }
                if let Some(v) = token.strip_prefix("port=") {
                    port = v.parse::<u16>().ok();
                }
                if let Some(v) = token.strip_prefix("tls-port=") {
                    tls_port = v.parse::<u16>().ok();
                    if port.is_none() {
                        port = tls_port;
                    }
                }
                if let Some(v) = token.strip_prefix("tls-ciphers=") {
                    let token = v.trim();
                    if !token.is_empty() {
                        tls_ciphers = Some(token.to_string());
                    }
                }
                if token == "disable-ticketing=on" {
                    disable_ticketing = true;
                }
                if token == "seamless-migration=on" {
                    seamless_migration = true;
                }
            }

            if let Some(port) = port {
                return Some(crate::runtime_model::Display::Spice {
                    spice: Spice::new(listen, port, disable_ticketing)
                        .with_gl_enabled(gl_enabled)
                        .with_tls_port(tls_port)
                        .with_tls_ciphers(tls_ciphers)
                        .with_seamless_migration(seamless_migration),
                });
            }
        }
    }

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-vnc"
        {
            if value == "none" {
                continue;
            }

            let mut password_auth = false;
            let target = value
                .split(',')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();

            for token in value.split(',').map(str::trim) {
                if token == "password=on" {
                    password_auth = true;
                }
            }

            if let Some(socket_path) = target.strip_prefix("unix:") {
                return Some(crate::runtime_model::Display::Vnc {
                    vnc: Vnc::new(String::new(), 0)
                        .with_gl_enabled(gl_enabled)
                        .with_socket_path(Some(socket_path.to_string()))
                        .with_password_auth(password_auth),
                });
            }

            if let Some((listen, port)) = target.rsplit_once(':')
                && let Ok(port) = port.parse::<u16>()
            {
                return Some(crate::runtime_model::Display::Vnc {
                    vnc: Vnc::new(listen.to_string(), port)
                        .with_gl_enabled(gl_enabled)
                        .with_password_auth(password_auth),
                });
            }
        }
    }

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-display"
        {
            if value.starts_with("egl-headless") {
                return Some(crate::runtime_model::Display::EglHeadless {
                    egl_headless: EglHeadless::default(),
                });
            }
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

    if gl_enabled {
        return Some(crate::runtime_model::Display::EglHeadless {
            egl_headless: EglHeadless::default(),
        });
    }

    None
}

fn parse_guest_agent(args: &[String]) -> Option<crate::runtime_model::GuestAgent> {
    if args
        .iter()
        .any(|arg| arg.contains("org.qemu.guest_agent.0"))
    {
        return Some(crate::runtime_model::GuestAgent { enabled: true });
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

fn parse_lifecycle(args: &[String]) -> Option<LifecycleConfig> {
    let daemonize = args.iter().any(|arg| arg == "-daemonize");
    let no_shutdown = args.iter().any(|arg| arg == "-no-shutdown");

    let mut pidfile: Option<String> = None;
    let mut qmp_socket: Option<String> = None;
    let mut qmp_event_socket: Option<String> = None;

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-pidfile"
        {
            let value = value.trim();
            if !value.is_empty() {
                pidfile = Some(value.to_string());
            }
        }

        if let [flag, value] = window
            && flag == "-chardev"
        {
            let mut id: Option<&str> = None;
            let mut path: Option<&str> = None;

            for token in value.split(',').map(str::trim) {
                if let Some(v) = token.strip_prefix("id=") {
                    id = Some(v);
                }
                if let Some(v) = token.strip_prefix("path=") {
                    path = Some(v);
                }
            }

            match (id, path) {
                (Some("qmp"), Some(path)) if !path.is_empty() => {
                    qmp_socket = Some(path.to_string())
                }
                (Some("qmp-event"), Some(path)) if !path.is_empty() => {
                    qmp_event_socket = Some(path.to_string())
                }
                _ => {}
            }
        }
    }

    if pidfile.is_none()
        && !daemonize
        && !no_shutdown
        && qmp_socket.is_none()
        && qmp_event_socket.is_none()
    {
        return None;
    }

    Some(LifecycleConfig::new(
        pidfile,
        daemonize,
        no_shutdown,
        qmp_socket,
        qmp_event_socket,
    ))
}

fn parse_balloon(args: &[String]) -> Option<BalloonConfig> {
    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-device"
            && value.starts_with("virtio-balloon-pci")
        {
            let mut free_page_reporting = false;
            for token in value.split(',').map(str::trim) {
                if let Some(v) = token.strip_prefix("free-page-reporting=") {
                    free_page_reporting = matches!(v, "on" | "1" | "true");
                }
            }

            return Some(BalloonConfig::new(true, free_page_reporting));
        }
    }

    None
}

fn parse_serial(args: &[String]) -> Option<SerialConfig> {
    let mut socket_path: Option<String> = None;
    let mut server = true;
    let mut wait = false;

    for window in args.windows(2) {
        if let [flag, value] = window
            && flag == "-chardev"
            && value.contains("id=serial0")
            && value.starts_with("socket")
        {
            for token in value.split(',').map(str::trim) {
                if let Some(v) = token.strip_prefix("path=")
                    && !v.is_empty()
                {
                    socket_path = Some(v.to_string());
                }
                if let Some(v) = token.strip_prefix("server=") {
                    server = matches!(v, "on" | "1" | "true");
                }
                if let Some(v) = token.strip_prefix("wait=") {
                    wait = matches!(v, "on" | "1" | "true");
                }
            }
        }
    }

    let serial_device_present = args.iter().any(|arg| {
        arg.contains("isa-serial") && arg.contains("chardev=serial0")
            || arg.contains("chardev:serial0")
    });
    if !serial_device_present {
        return None;
    }

    Some(SerialConfig::new(socket_path, server, wait))
}

// ---------------------------------------------------------------------------
// GPU registration
// ---------------------------------------------------------------------------

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

fn register_gpu_from_args(args: &[String], busses: &BusRegister) -> Result<(), String> {
    use crate::runtime_model::{
        PciDeviceApi, PciDeviceType, PcieAddress, PcieDeviceApi, PcieDeviceType,
    };
    use std::sync::Arc;

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

#[cfg(test)]
mod tests {
    use crate::config_format::{
        Parser, RuntimeBuilder,
        qemu_cmd::{parser::QemuParser, runtime_render::render_qemu_command},
    };

    use super::QemuRuntimeBuilder;

    #[test]
    fn imports_display_guest_agent_and_gpu_from_args() {
        let cmd = "qemu-system-x86_64 -name vm1 -m 4096 -cpu host -smp 4,sockets=1,cores=4,threads=1 -spice port=5905,addr=127.0.0.1,disable-ticketing=on -device virtio-vga -chardev socket,path=/tmp/vm1.qga,server=on,wait=off,id=qga0 -device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .expect("runtime build should succeed");

        let rendered = render_qemu_command(&runtime).expect("qemu render should succeed");
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
    fn imports_vnc_socket_gl_and_virtio_vga_gl_from_args() {
        let cmd = "qemu-system-x86_64 -name vm-gl -m 4096 -cpu host -smp 4,sockets=1,cores=4,threads=1 -display egl-headless,gl=core -vnc unix:/var/run/ezkvm/vm-gl.vnc,password=on -device virtio-vga-gl";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .expect("runtime build should succeed");

        let rendered = render_qemu_command(&runtime).expect("qemu render should succeed");
        assert!(
            rendered
                .iter()
                .any(|arg| arg == "egl-headless,gl=core" || arg.contains("egl-headless,gl=core"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("virtio-vga-gl")));
        assert!(rendered.iter().any(
            |arg| arg.contains("unix:/var/run/ezkvm/vm-gl.vnc") && arg.contains("password=on")
        ));
    }

    #[test]
    fn imports_spice_tls_options_from_args() {
        let cmd = "qemu-system-x86_64 -name vm-spice -m 4096 -cpu host -smp 4,sockets=1,cores=4,threads=1 -spice port=5905,tls-port=61005,addr=127.0.0.1,tls-ciphers=HIGH,seamless-migration=on,disable-ticketing=on";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .expect("runtime build should succeed");

        let rendered = render_qemu_command(&runtime).expect("qemu render should succeed");
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("port=5905") && arg.contains("tls-port=61005"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("tls-ciphers=HIGH")));
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("seamless-migration=on"))
        );
    }

    #[test]
    fn imports_qxl_gpu_from_vga_flag() {
        let cmd = "qemu-system-x86_64 -name vm2 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1 -vga qxl";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .expect("runtime build should succeed");

        let rendered = render_qemu_command(&runtime).expect("qemu render should succeed");
        assert!(rendered.iter().any(|arg| arg == "qxl" || arg == "VGA"));
    }

    #[test]
    fn imports_identity_fields_from_args() {
        let cmd = "qemu-system-x86_64 -name vm3 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1 -smbios type=1,uuid=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee -device vmgenid,guid=11111111-2222-3333-4444-555555555555";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
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

    #[test]
    fn imports_lifecycle_and_qmp_monitoring_from_args() {
        let cmd = "qemu-system-x86_64 -name vm-lifecycle -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1 -pidfile /run/qemu/vm-lifecycle.pid -daemonize -no-shutdown -chardev socket,id=qmp,path=/run/qemu/vm-lifecycle.qmp,server=on,wait=off -mon chardev=qmp,mode=control -chardev socket,id=qmp-event,path=/run/qemu/vm-lifecycle.event,server=on,wait=off -mon chardev=qmp-event,mode=control";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .expect("runtime build should succeed");

        let lifecycle = runtime
            .lifecycle_config()
            .as_ref()
            .expect("lifecycle config should be imported");
        assert_eq!(
            lifecycle.pidfile().as_deref(),
            Some("/run/qemu/vm-lifecycle.pid")
        );
        assert!(*lifecycle.daemonize());
        assert!(*lifecycle.no_shutdown());
        assert_eq!(
            lifecycle.qmp_socket().as_deref(),
            Some("/run/qemu/vm-lifecycle.qmp")
        );
        assert_eq!(
            lifecycle.qmp_event_socket().as_deref(),
            Some("/run/qemu/vm-lifecycle.event")
        );

        let rendered = render_qemu_command(&runtime).expect("qemu render should succeed");
        assert!(rendered.iter().any(|arg| arg == "-daemonize"));
        assert!(rendered.iter().any(|arg| arg == "-no-shutdown"));
        assert!(
            rendered
                .iter()
                .any(|arg| arg == "/run/qemu/vm-lifecycle.pid")
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("socket,id=qmp,path=/run/qemu/vm-lifecycle.qmp"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("socket,id=qmp-event,path=/run/qemu/vm-lifecycle.event"))
        );
    }

    #[test]
    fn imports_balloon_and_serial_from_args() {
        let cmd = "qemu-system-x86_64 -name vm-devices -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1 -device virtio-balloon-pci,id=balloon0,free-page-reporting=on -chardev socket,id=serial0,path=/run/qemu/vm-devices.serial0,server=on,wait=off -device isa-serial,chardev=serial0";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .expect("runtime build should succeed");

        let balloon = runtime
            .balloon_config()
            .as_ref()
            .expect("balloon config should be imported");
        assert!(*balloon.enabled());
        assert!(*balloon.free_page_reporting());

        let serial = runtime
            .serial_config()
            .as_ref()
            .expect("serial config should be imported");
        assert_eq!(
            serial.socket_path().as_deref(),
            Some("/run/qemu/vm-devices.serial0")
        );
        assert!(*serial.server());
        assert!(!*serial.wait());

        let rendered = render_qemu_command(&runtime).expect("qemu render should succeed");
        assert!(rendered.iter().any(
            |arg| arg.contains("virtio-balloon-pci") && arg.contains("free-page-reporting=on")
        ));
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("isa-serial,chardev=serial0"))
        );
    }

    #[test]
    fn imports_cpu_features_power_globals_and_iscsi_from_args() {
        let cmd = "qemu-system-x86_64 -name vm-phase4 -m 2048 -cpu Skylake-Client,+kvm_pv_eoi,-vmx -smp 2,sockets=1,cores=2,threads=1 -global ICH9-LPC.disable_s3=1 -global ICH9-LPC.disable_s4=0 -iscsi initiator-name=iqn.1993-08.org.debian:01:c58d3b2cb8dd";
        let schema = QemuParser.parse(cmd).expect("command should parse");

        let runtime = QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .expect("runtime build should succeed");

        match runtime.cpu().model() {
            crate::runtime_model::CpuModel::Named { name } => {
                assert_eq!(name, "Skylake-Client")
            }
            _ => panic!("expected named CPU model"),
        }
        assert!(
            runtime
                .cpu()
                .enabled_features()
                .iter()
                .any(|feature| feature == "kvm_pv_eoi")
        );
        assert!(
            runtime
                .cpu()
                .disabled_features()
                .iter()
                .any(|feature| feature == "vmx")
        );
        assert_eq!(
            runtime.iscsi_initiator().as_deref(),
            Some("iqn.1993-08.org.debian:01:c58d3b2cb8dd")
        );

        let power = runtime
            .power_management_config()
            .as_ref()
            .expect("power management should be imported");
        assert_eq!(power.disable_s3(), &Some(true));
        assert_eq!(power.disable_s4(), &Some(false));
    }
}
