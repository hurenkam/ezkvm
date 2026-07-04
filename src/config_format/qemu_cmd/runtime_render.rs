use crate::runtime_model::{
    Ac97Controller, AudioBackend, AudioController, BiosModel, Chipset, Cpu, CpuModel, Display,
    Ich9IntelHdaController, IvshmemPlainController, Memory, NetworkResource,
    PassthroughGpuController, PassthroughPcieController, PcieAddress, PcieDeviceApi,
    PvScsiController, QxlGpuController, RuntimeModel, ScsiAddress, ScsiControllerApi,
    StandardGpuController, StorageDeviceKind, StorageResource, UsbHostByBusPortController,
    UsbHostByIdController, UsbTabletController, VirtioGpuController, VirtioNetController,
};

#[derive(Debug, Clone, Default)]
struct StorageThrottleLimits {
    bps_total: Option<u64>,
    bps_read: Option<u64>,
    bps_write: Option<u64>,
    iops_total: Option<u64>,
    iops_read: Option<u64>,
    iops_write: Option<u64>,
}

#[derive(Debug, Clone)]
struct ThrottleGroupConfig {
    id: String,
    limits: StorageThrottleLimits,
}

#[derive(Debug, Clone)]
struct StorageBackendConfig {
    backend_id: String,
    file_node_name: String,
    node_name: String,
    file_path: String,
    read_only: bool,
    detect_zeroes_unmap: bool,
    discard_unmap: bool,
    throttle_group: Option<ThrottleGroupConfig>,
}

const OVMF_CODE_PATH: &str = "/usr/share/pve-edk2-firmware/OVMF_CODE_4M.secboot.fd";
const OVMF_VARS_SIZE: usize = 540_672;

pub fn render_qemu_command(runtime: &RuntimeModel) -> Result<Vec<String>, String> {
    let mut args = vec![
        "qemu-system-x86_64".to_string(),
        "-name".to_string(),
        runtime.name().clone(),
    ];

    args.extend(render_cpu_args(runtime.cpu()));
    args.extend(render_memory_args(runtime.memory(), runtime.cpu()));
    args.extend(render_chipset_args(runtime.chipset())?);
    args.extend(render_boot_args(runtime));

    if let Some(smbios_uuid) = runtime.smbios_uuid() {
        args.extend(["-smbios".to_string(), format!("type=1,uuid={smbios_uuid}")]);
    }
    if let Some(vmgenid) = runtime.vmgenid() {
        args.extend(["-device".to_string(), format!("vmgenid,guid={vmgenid}")]);
    }
    if let Some(display) = runtime.display() {
        args.extend(render_display_args(display.config()));
    }
    if let Some(audio) = runtime.audio() {
        args.extend(render_audio_args(audio.config()));
    }
    if let Some(tpm) = runtime.tpm()
        && tpm.swtpm_version().is_some()
    {
        let vm_name = runtime.name();
        let socket_path = format!("/var/run/ezkvm/{vm_name}.swtpm");
        args.extend([
            "-chardev".to_string(),
            format!("socket,id=tpmchar,path={socket_path}"),
            "-tpmdev".to_string(),
            "emulator,id=tpmdev,chardev=tpmchar".to_string(),
            "-device".to_string(),
            "tpm-tis,tpmdev=tpmdev".to_string(),
        ]);
    }
    if let Some(guest_agent) = runtime.guest_agent()
        && guest_agent.config().enabled
    {
        let socket_path = format!("/var/run/ezkvm/{}.agent", runtime.name());
        args.extend([
            "-chardev".to_string(),
            format!("socket,path={socket_path},server=on,wait=off,id=qga0"),
            "-device".to_string(),
            "virtio-serial".to_string(),
            "-device".to_string(),
            "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0".to_string(),
        ]);
    }

    args.extend(render_lifecycle_args(runtime));

    args.extend(render_bus_args(runtime));

    Ok(args)
}

pub fn render_memory_args(memory: &Memory, cpu: &Cpu) -> Vec<String> {
    let size_mb = memory.size() / 1024 / 1024;
    let mut args = vec!["-m".to_string(), format!("{size_mb}M")];

    if let Some(hugepages_kb) = memory.hugepages_kb() {
        args.extend([
            "-object".to_string(),
            format!(
                "memory-backend-file,id=ram-node0,size={size_mb}M,mem-path=/run/hugepages/kvm/{hugepages_kb}kB,share=on,prealloc=yes"
            ),
        ]);
    }

    if memory.numa_enabled() {
        let total_vcpus = cpu.total_vcpus().max(1);
        let cpus = if total_vcpus == 1 {
            "0".to_string()
        } else {
            format!("0-{}", total_vcpus - 1)
        };

        let node = if memory.hugepages_kb().is_some() {
            format!("node,nodeid=0,cpus={cpus},memdev=ram-node0")
        } else {
            format!("node,nodeid=0,cpus={cpus},mem={size_mb}")
        };

        args.extend(["-numa".to_string(), node]);
    }

    args
}

fn render_cpu_args(cpu: &Cpu) -> Vec<String> {
    let model_name = match cpu.model() {
        CpuModel::Host => "host",
    };

    let sockets = cpu.sockets().max(1);
    let cores = cpu.cores().max(1);
    let threads = cpu.threads().max(1);
    let total_vcpus = cpu.total_vcpus();

    vec![
        "-cpu".to_string(),
        model_name.to_string(),
        "-smp".to_string(),
        format!("{total_vcpus},sockets={sockets},cores={cores},threads={threads}"),
    ]
}

fn render_chipset_args(chipset: &Chipset) -> Result<Vec<String>, String> {
    match chipset {
        Chipset::Q35(_) => Ok(vec![
            "-machine".to_string(),
            "type=q35".to_string(),
            "-readconfig".to_string(),
            "/usr/share/qemu-server/pve-q35-4.0.cfg".to_string(),
            "-nodefaults".to_string(),
        ]),
        Chipset::I440FX(_) => Err("I440FX qemu rendering is not implemented yet".to_string()),
    }
}

fn render_boot_args(runtime: &RuntimeModel) -> Vec<String> {
    let mut args = vec![
        "-boot".to_string(),
        "menu=on,strict=on,reboot-timeout=1000".to_string(),
    ];

    match runtime.boot().bios() {
        BiosModel::SeaBios(_) => {
            args.extend(["-bios".to_string(), "/usr/share/qemu/bios.bin".to_string()]);
        }
        BiosModel::Uefi(uefi) => {
            let vars_path = match uefi.storage() {
                StorageResource::File { file } => file.clone(),
                StorageResource::BlockDevice { block_device } => block_device.clone(),
            };

            args.extend([
                "-drive".to_string(),
                format!(
                    "if=pflash,unit=0,format=raw,readonly=on,file={}",
                    OVMF_CODE_PATH
                ),
                "-drive".to_string(),
                format!(
                    "if=pflash,unit=1,id=drive-efidisk0,format=raw,file={},size={}",
                    vars_path, OVMF_VARS_SIZE
                ),
            ]);
        }
    }

    args
}

fn render_display_args(display: &Display) -> Vec<String> {
    match display {
        Display::Gtk { .. } => vec!["-display".to_string(), "gtk".to_string()],
        Display::Sdl { .. } => vec!["-display".to_string(), "sdl".to_string()],
        Display::Vnc { vnc } => {
            let mut args = Vec::new();
            if *vnc.gl_enabled() {
                args.extend(["-display".to_string(), "egl-headless,gl=core".to_string()]);
            }
            if let Some(socket_path) = vnc.socket_path() {
                let mut value = format!("unix:{socket_path}");
                if *vnc.password_auth() {
                    value.push_str(",password=on");
                }
                args.extend(["-vnc".to_string(), value]);
            } else {
                let listen = if vnc.listen().is_empty() {
                    "0.0.0.0"
                } else {
                    vnc.listen()
                };
                let mut value = format!("{}:{}", listen, vnc.port());
                if *vnc.password_auth() {
                    value.push_str(",password=on");
                }
                args.extend(["-vnc".to_string(), value]);
            }
            args
        }
        Display::Spice { spice } => {
            let mut args = Vec::new();
            if *spice.gl_enabled() {
                args.extend(["-display".to_string(), "egl-headless,gl=core".to_string()]);
            }
            let listen = if spice.listen().is_empty() {
                "0.0.0.0"
            } else {
                spice.listen()
            };
            let mut spec = if let Some(tls_port) = spice.tls_port() {
                format!(
                    "port={},tls-port={},addr={}",
                    spice.port(),
                    tls_port,
                    listen
                )
            } else {
                format!("port={},addr={}", spice.port(), listen)
            };
            if *spice.disable_ticketing() {
                spec.push_str(",disable-ticketing=on");
            }
            if let Some(tls_ciphers) = spice.tls_ciphers() {
                spec.push_str(&format!(",tls-ciphers={tls_ciphers}"));
            }
            if *spice.seamless_migration() {
                spec.push_str(",seamless-migration=on");
            }
            args.extend(["-spice".to_string(), spec]);
            args
        }
        Display::EglHeadless { .. } => {
            vec!["-display".to_string(), "egl-headless,gl=core".to_string()]
        }
        Display::LookingGlass { .. } => vec![
            "-display".to_string(),
            "none".to_string(),
            "-vnc".to_string(),
            "none".to_string(),
        ],
    }
}

fn render_audio_args(audio: &crate::runtime_model::Audio) -> Vec<String> {
    let backend_id = "audio0";
    let backend_str = match &audio.backend {
        AudioBackend::None => return vec![],
        AudioBackend::Alsa => "alsa",
        AudioBackend::PulseAudio => "pa",
        AudioBackend::PipeWire => "pipewire",
    };

    let mut args = vec![
        "-audiodev".to_string(),
        format!("{},id={}", backend_str, backend_id),
    ];

    match &audio.controller {
        AudioController::Ich9IntelHda => {
            args.extend([
                "-device".to_string(),
                "ich9-intel-hda,id=sound0".to_string(),
                "-device".to_string(),
                format!(
                    "hda-duplex,id=sound0-codec0,bus=sound0.0,cad=0,audiodev={}",
                    backend_id
                ),
            ]);
        }
        AudioController::Ac97 => {
            args.extend([
                "-device".to_string(),
                format!("AC97,audiodev={}", backend_id),
            ]);
        }
    }

    args
}

fn render_lifecycle_args(runtime: &RuntimeModel) -> Vec<String> {
    let Some(config) = runtime.lifecycle_config().as_ref() else {
        return Vec::new();
    };

    let mut args = Vec::new();

    if let Some(pidfile) = config.pidfile() {
        args.extend(["-pidfile".to_string(), pidfile.clone()]);
    }
    if *config.daemonize() {
        args.push("-daemonize".to_string());
    }
    if *config.no_shutdown() {
        args.push("-no-shutdown".to_string());
    }

    if let Some(socket_path) = config.qmp_socket() {
        args.extend([
            "-chardev".to_string(),
            format!("socket,id=qmp,path={socket_path},server=on,wait=off"),
            "-mon".to_string(),
            "chardev=qmp,mode=control".to_string(),
        ]);
    }

    if let Some(socket_path) = config.qmp_event_socket() {
        args.extend([
            "-chardev".to_string(),
            format!("socket,id=qmp-event,path={socket_path},server=on,wait=off"),
            "-mon".to_string(),
            "chardev=qmp-event,mode=control".to_string(),
        ]);
    }

    args
}

fn render_bus_args(runtime: &RuntimeModel) -> Vec<String> {
    let mut args = Vec::new();
    let display = runtime.display().as_ref().map(|display| display.config());

    let mut pci_bus_ids: Vec<_> = runtime.busses().pci_busses().keys().copied().collect();
    pci_bus_ids.sort_unstable();
    for bus_id in pci_bus_ids {
        if let Some(controller) = runtime.busses().pci_busses().get(&bus_id) {
            let mut entries: Vec<_> = controller.devices().into_iter().collect();
            entries.sort_by_key(|(address, _)| (address.device, address.function));
            for (_address, device) in entries {
                args.extend(render_pci_device(bus_id, device.as_ref()));
            }
        }
    }

    let mut pcie_bus_ids: Vec<_> = runtime.busses().pcie_busses().keys().copied().collect();
    pcie_bus_ids.sort_unstable();
    for bus_id in pcie_bus_ids {
        if let Some(controller) = runtime.busses().pcie_busses().get(&bus_id) {
            let devices = controller.devices();
            let mut addresses: Vec<_> = devices.keys().cloned().collect();
            addresses.sort_by_key(|address| (address.device(), address.function()));
            for address in addresses {
                if let Some(device) = devices.get(&address) {
                    args.extend(render_pcie_device(
                        bus_id,
                        address.clone(),
                        device.as_ref(),
                        display,
                    ));
                }
            }
        }
    }

    let mut usb_bus_ids: Vec<_> = runtime.busses().usb_busses().keys().copied().collect();
    usb_bus_ids.sort_unstable();
    for bus_id in usb_bus_ids {
        if let Some(controller) = runtime.busses().usb_busses().get(&bus_id) {
            args.extend(render_usb_bus_args(bus_id, controller.as_ref()));
        }
    }

    let mut sata_bus_ids: Vec<_> = runtime.busses().sata_busses().keys().copied().collect();
    sata_bus_ids.sort_unstable();
    for bus_id in sata_bus_ids {
        if let Some(controller) = runtime.busses().sata_busses().get(&bus_id) {
            if bus_id == 0 {
                args.extend([
                    "-device".to_string(),
                    "ahci,id=ahci0,bus=pcie.0,addr=0x7".to_string(),
                ]);
            }
            let devices = controller.devices();
            let mut addresses: Vec<_> = devices.keys().cloned().collect();
            addresses.sort_by_key(|address| address.address);
            for address in addresses {
                if let Some(device) = devices.get(&address) {
                    args.extend(render_sata_drive_args(
                        device.storage_resource(),
                        address.address,
                    ));
                }
            }
        }
    }

    let mut ide_bus_ids: Vec<_> = runtime.busses().ide_busses().keys().copied().collect();
    ide_bus_ids.sort_unstable();
    for bus_id in ide_bus_ids {
        if let Some(controller) = runtime.busses().ide_busses().get(&bus_id) {
            let devices = controller.devices();
            let mut addresses: Vec<_> = devices.keys().cloned().collect();
            addresses.sort_by_key(|address| address.address);
            for address in addresses {
                if let Some(device) = devices.get(&address) {
                    args.extend(render_ide_drive_args(
                        device.storage_resource(),
                        1,
                        address.address,
                        device.storage_kind(),
                    ));
                }
            }
        }
    }

    args
}

fn render_pci_device(_bus: u8, device: &dyn crate::runtime_model::PciDeviceApi) -> Vec<String> {
    if device.as_any().is::<QxlGpuController>() {
        return vec!["-device".to_string(), "qxl".to_string()];
    }

    if device.as_any().is::<Ac97Controller>() {
        return vec!["-device".to_string(), "AC97".to_string()];
    }

    Vec::new()
}

fn render_pcie_device(
    bus: u8,
    address: PcieAddress,
    device: &dyn PcieDeviceApi,
    display: Option<&Display>,
) -> Vec<String> {
    if let Some(net) = device.as_any().downcast_ref::<VirtioNetController>() {
        return render_virtio_net_device(bus, address, net);
    }

    if let Some(controller) = device.as_any().downcast_ref::<PvScsiController>() {
        return render_pvscsi_device(bus, address, controller);
    }

    if let Some(passthrough) = device.as_any().downcast_ref::<PassthroughPcieController>() {
        let mut value = format!("vfio-pci,host={}", passthrough.host());
        if let Some(id) = passthrough.id() {
            value.push_str(&format!(",id={id}"));
        }
        if let Some(multifunction) = passthrough.multifunction() {
            value.push_str(&format!(
                ",multifunction={}",
                if multifunction { 1 } else { 0 }
            ));
        }
        if let Some(rombar) = passthrough.rombar() {
            value.push_str(&format!(",rombar={}", if rombar { 1 } else { 0 }));
        }
        if let Some(romfile) = passthrough.romfile() {
            value.push_str(&format!(",romfile={romfile}"));
        }
        value.push_str(&format!(
            ",bus=pcie.{bus},addr={}",
            format_pcie_addr(&address)
        ));
        return vec!["-device".to_string(), value];
    }

    if let Some(gpu) = device.as_any().downcast_ref::<PassthroughGpuController>() {
        return vec![
            "-device".to_string(),
            format!("vfio-pci,id={}", gpu.resource()),
        ];
    }

    if device.as_any().is::<StandardGpuController>() {
        return vec!["-device".to_string(), "VGA".to_string()];
    }

    if device.as_any().is::<VirtioGpuController>() {
        if display_uses_gl(display) {
            return vec![
                "-device".to_string(),
                "virtio-vga-gl,id=vga,max_hostmem=67108864".to_string(),
            ];
        }
        return vec!["-device".to_string(), "virtio-gpu-pci".to_string()];
    }

    if let Some(hda) = device.as_any().downcast_ref::<Ich9IntelHdaController>() {
        let mut args = vec!["-device".to_string(), "ich9-intel-hda".to_string()];
        if let Some(codec) = hda.codec() {
            args.push("-device".to_string());
            args.push(codec.to_string());
        }
        return args;
    }

    if let Some(ivshmem) = device.as_any().downcast_ref::<IvshmemPlainController>() {
        return vec![
            "-device".to_string(),
            format!("ivshmem-plain,id={}", ivshmem.resource()),
        ];
    }

    Vec::new()
}

fn display_uses_gl(display: Option<&Display>) -> bool {
    match display {
        Some(Display::EglHeadless { .. }) => true,
        Some(Display::Vnc { vnc }) => *vnc.gl_enabled(),
        Some(Display::Spice { spice }) => *spice.gl_enabled(),
        _ => false,
    }
}

fn render_virtio_net_device(
    bus: u8,
    address: PcieAddress,
    net: &VirtioNetController,
) -> Vec<String> {
    let device_id = format!("net{}f{}", address.device(), address.function());
    let netdev = match net.resource() {
        Some(NetworkResource::Tap { tap }) => {
            let mut value = format!("tap,id={device_id},ifname={tap}");
            if let Some(vhost) = net.vhost() {
                value.push_str(&format!(",vhost={}", if vhost { "on" } else { "off" }));
            }
            value
        }
        Some(NetworkResource::Bridge { bridge }) => {
            format!("bridge,id={device_id},br={bridge}")
        }
        None => format!("user,id={device_id}"),
    };

    let mut device_fields = vec![
        "virtio-net-pci".to_string(),
        format!("id={device_id}"),
        format!("netdev={device_id}"),
        format!("bus=pcie.{bus}"),
        format!("addr={}", format_pcie_addr(&address)),
    ];

    if let Some(mac) = net.mac_address() {
        device_fields.push(format!("mac={mac}"));
    }
    if let Some(rx_queue_size) = net.rx_queue_size() {
        device_fields.push(format!("rx_queue_size={rx_queue_size}"));
    }
    if let Some(tx_queue_size) = net.tx_queue_size() {
        device_fields.push(format!("tx_queue_size={tx_queue_size}"));
    }

    vec![
        "-netdev".to_string(),
        netdev,
        "-device".to_string(),
        device_fields.join(","),
    ]
}

fn render_pvscsi_device(
    bus: u8,
    address: PcieAddress,
    controller: &PvScsiController,
) -> Vec<String> {
    let mut args = vec![
        "-device".to_string(),
        format!(
            "pvscsi,id=scsihw0,bus=pcie.{bus},addr={}",
            format_pcie_addr(&address)
        ),
    ];

    let devices = controller.devices();
    let mut addresses: Vec<_> = devices.keys().cloned().collect();
    addresses.sort_by_key(|a| (a.target, a.lun));
    for scsi_addr in addresses {
        if let Some(device) = devices.get(&scsi_addr) {
            args.extend(render_scsi_drive_args(
                device.storage_resource(),
                0,
                scsi_addr,
                device.storage_kind(),
            ));
        }
    }

    args
}

fn render_usb_bus_args(
    bus: u8,
    controller: &dyn crate::runtime_model::UsbControllerApi,
) -> Vec<String> {
    let devices = controller.devices();
    if devices.is_empty() {
        return Vec::new();
    }

    let controller_id = format!("xhci{bus}");
    let controller_addr = format!("0x{:x}", 0x1b_u8.saturating_add(bus));

    let mut args = vec![
        "-device".to_string(),
        format!("qemu-xhci,p2=15,p3=15,id={controller_id},bus=pcie.0,addr={controller_addr}"),
    ];

    let mut addresses: Vec<_> = devices.keys().cloned().collect();
    addresses.sort_by_key(|address| address.port.clone());
    for address in addresses {
        if let Some(device) = devices.get(&address) {
            args.extend(render_usb_device_args(
                bus,
                controller_id.as_str(),
                &address,
                device.as_ref(),
            ));
        }
    }

    args
}

fn render_usb_device_args(
    bus: u8,
    controller_id: &str,
    address: &crate::runtime_model::UsbAddress,
    device: &dyn crate::runtime_model::UsbDeviceApi,
) -> Vec<String> {
    let port = format_usb_port(&address.port);
    let id_suffix = address
        .port
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();

    if device
        .as_any()
        .downcast_ref::<UsbTabletController>()
        .is_some()
    {
        return vec![
            "-device".to_string(),
            format!("usb-tablet,id=tablet-b{bus}-{id_suffix},bus={controller_id}.0,port={port}"),
        ];
    }

    if let Some(host) = device.as_any().downcast_ref::<UsbHostByBusPortController>() {
        return vec![
            "-device".to_string(),
            format!(
                "usb-host,id=usb-b{bus}-{id_suffix},bus={controller_id}.0,port={port},hostbus={},hostport={}",
                host.hostbus(),
                host.hostport()
            ),
        ];
    }

    if let Some(host) = device.as_any().downcast_ref::<UsbHostByIdController>() {
        return vec![
            "-device".to_string(),
            format!(
                "usb-host,id=usb-b{bus}-{id_suffix},bus={controller_id}.0,port={port},vendorid=0x{:04x},productid=0x{:04x}",
                host.vendor_id(),
                host.device_id()
            ),
        ];
    }

    Vec::new()
}

fn format_usb_port(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        return "1".to_string();
    }
    if let Some(stripped) = value.strip_prefix("usb")
        && !stripped.trim().is_empty()
    {
        return stripped.trim().to_string();
    }
    value.to_string()
}

fn render_ide_drive_args(
    resource: &StorageResource,
    bus: u8,
    unit: u8,
    kind: StorageDeviceKind,
) -> Vec<String> {
    let backend_id = format!("drive-ide{unit}");
    let device_id = format!("ide{unit}");
    let device_type = match kind {
        StorageDeviceKind::Cdrom => "ide-cd",
        StorageDeviceKind::Hdd | StorageDeviceKind::Ssd => "ide-hd",
    };

    let backend = build_storage_backend_config(backend_id, resource, kind, false, false, None);

    let mut args = render_storage_backend_args(&backend);
    args.extend([
        "-device".to_string(),
        format!(
            "{device_type},bus=ide.{bus},unit={unit},drive={},id={device_id}",
            backend.node_name
        ),
    ]);

    args
}

fn render_sata_drive_args(resource: &StorageResource, address: u8) -> Vec<String> {
    let backend_id = format!("drive-sata{address}");
    let device_id = format!("sata{address}");
    let backend = build_storage_backend_config(
        backend_id,
        resource,
        StorageDeviceKind::Hdd,
        true,
        true,
        None,
    );

    let mut args = render_storage_backend_args(&backend);
    args.extend([
        "-device".to_string(),
        format!(
            "ide-hd,id={device_id},drive={},bus=ahci0.{address}",
            backend.node_name
        ),
    ]);

    args
}

fn render_scsi_drive_args(
    resource: &StorageResource,
    bus: u8,
    address: ScsiAddress,
    kind: StorageDeviceKind,
) -> Vec<String> {
    let backend_id = format!("drive-scsi{}", address.lun);
    let device_id = format!("scsi{}", address.lun);

    let device_type = match kind {
        StorageDeviceKind::Cdrom => "scsi-cd",
        StorageDeviceKind::Hdd | StorageDeviceKind::Ssd => "scsi-hd",
    };

    let backend = build_storage_backend_config(backend_id, resource, kind, true, true, None);

    let mut args = render_storage_backend_args(&backend);
    args.extend([
        "-device".to_string(),
        format!(
            "{device_type},bus=scsihw{bus}.0,channel=0,scsi-id={},lun={},drive={},id={device_id}",
            address.target, address.lun, backend.node_name
        ),
    ]);

    args
}

fn build_storage_backend_config(
    backend_id: String,
    resource: &StorageResource,
    kind: StorageDeviceKind,
    discard_unmap: bool,
    detect_zeroes_unmap: bool,
    throttle_group: Option<ThrottleGroupConfig>,
) -> StorageBackendConfig {
    let file_path = match resource {
        StorageResource::File { file } => file.clone(),
        StorageResource::BlockDevice { block_device } => block_device.clone(),
    };

    StorageBackendConfig {
        file_node_name: format!("file-{backend_id}"),
        node_name: format!("node-{backend_id}"),
        backend_id,
        file_path,
        read_only: kind == StorageDeviceKind::Cdrom,
        detect_zeroes_unmap,
        discard_unmap,
        throttle_group,
    }
}

fn render_storage_backend_args(config: &StorageBackendConfig) -> Vec<String> {
    let mut args = vec![
        "-blockdev".to_string(),
        format!(
            "driver=file,node-name={},filename={}",
            config.file_node_name, config.file_path
        ),
    ];

    let mut backend_fields = vec![
        "driver=raw".to_string(),
        format!("id={}", config.backend_id),
        format!("node-name={}", config.node_name),
        format!("file={}", config.file_node_name),
    ];

    if config.read_only {
        backend_fields.push("read-only=on".to_string());
    }
    if config.discard_unmap {
        backend_fields.push("discard=unmap".to_string());
    }
    if config.detect_zeroes_unmap {
        backend_fields.push("detect-zeroes=unmap".to_string());
    }
    if let Some(group) = &config.throttle_group {
        backend_fields.push(format!("throttle-group={}", group.id));
    }

    args.extend(["-blockdev".to_string(), backend_fields.join(",")]);

    if let Some(group) = &config.throttle_group {
        args.extend(render_throttle_group_args(group));
    }

    args
}

fn render_throttle_group_args(group: &ThrottleGroupConfig) -> Vec<String> {
    let mut fields = vec![format!("throttle-group,id={}", group.id)];

    if let Some(value) = group.limits.bps_total {
        fields.push(format!("x-bps-total={value}"));
    }
    if let Some(value) = group.limits.bps_read {
        fields.push(format!("x-bps-read={value}"));
    }
    if let Some(value) = group.limits.bps_write {
        fields.push(format!("x-bps-write={value}"));
    }
    if let Some(value) = group.limits.iops_total {
        fields.push(format!("x-iops-total={value}"));
    }
    if let Some(value) = group.limits.iops_read {
        fields.push(format!("x-iops-read={value}"));
    }
    if let Some(value) = group.limits.iops_write {
        fields.push(format!("x-iops-write={value}"));
    }

    vec!["-object".to_string(), fields.join(",")]
}

fn format_pcie_addr(address: &PcieAddress) -> String {
    if address.function() == 0 {
        format!("0x{:x}", address.device())
    } else {
        format!("0x{:x}.{}", address.device(), address.function())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::config_format::qemu_cmd::runtime_render::render_qemu_command;
    use crate::runtime_model::{
        BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, Display, DisplayModelBuilder,
        LifecycleConfig, Memory, PcieAddress, PcieDeviceApi, Q35Chipset, Q35UsbController,
        RuntimeModel, ScsiAddress, SeaBiosModel, StorageDeviceKind, StorageResource, UsbAddress,
        UsbControllerApi, UsbDeviceApi, UsbHostByBusPortController, UsbHostByIdController,
        UsbTabletController, VirtioGpuController,
    };

    use super::{
        StorageThrottleLimits, ThrottleGroupConfig, build_storage_backend_config,
        render_scsi_drive_args, render_storage_backend_args, render_usb_bus_args,
    };

    #[test]
    fn renders_scsi_disks_using_blockdev_nodes() {
        let args = render_scsi_drive_args(
            &StorageResource::File {
                file: "/var/lib/vm/disk0.raw".to_string(),
            },
            0,
            ScsiAddress::new(0, 0),
            StorageDeviceKind::Ssd,
        );

        assert!(args.iter().any(|arg| arg == "-blockdev"));
        assert!(args.iter().any(|arg| arg.contains("driver=file")));
        assert!(args.iter().any(|arg| arg.contains("driver=raw")));
        assert!(
            args.iter()
                .any(|arg| arg.contains("scsi-hd") && arg.contains("drive=node-drive-scsi0"))
        );
    }

    #[test]
    fn renders_optional_throttle_group_object_when_configured() {
        let config = build_storage_backend_config(
            "drive-scsi2".to_string(),
            &StorageResource::BlockDevice {
                block_device: "/dev/zvol/tank/vm-200-disk-0".to_string(),
            },
            StorageDeviceKind::Hdd,
            true,
            true,
            Some(ThrottleGroupConfig {
                id: "tg-scsi2".to_string(),
                limits: StorageThrottleLimits {
                    bps_total: Some(104857600),
                    ..StorageThrottleLimits::default()
                },
            }),
        );

        let args = render_storage_backend_args(&config);
        assert!(args.iter().any(|arg| arg == "-object"));
        assert!(
            args.iter()
                .any(|arg| arg.contains("throttle-group,id=tg-scsi2"))
        );
        assert!(args.iter().any(|arg| arg.contains("x-bps-total=104857600")));
        assert!(
            args.iter()
                .any(|arg| arg.contains("throttle-group=tg-scsi2"))
        );
    }

    #[test]
    fn renders_usb_controller_and_host_devices() {
        let controller = Q35UsbController::default();

        let tablet: Arc<dyn UsbDeviceApi> = Arc::new(UsbTabletController {});
        controller
            .register_usb_device(tablet, Some(UsbAddress::new("1".to_string())))
            .expect("tablet registration should succeed");

        let host_by_port: Arc<dyn UsbDeviceApi> =
            Arc::new(UsbHostByBusPortController::new(1, "7.5.1".to_string()));
        controller
            .register_usb_device(host_by_port, Some(UsbAddress::new("5".to_string())))
            .expect("usb host (bus/port) registration should succeed");

        let host_by_id: Arc<dyn UsbDeviceApi> =
            Arc::new(UsbHostByIdController::new(0x0451, 0x16a0));
        controller
            .register_usb_device(host_by_id, Some(UsbAddress::new("6".to_string())))
            .expect("usb host (vendor/product) registration should succeed");

        let args = render_usb_bus_args(0, &controller);

        assert!(
            args.iter()
                .any(|arg| arg.contains("qemu-xhci") && arg.contains("id=xhci0"))
        );
        assert!(
            args.iter()
                .any(|arg| arg.contains("usb-tablet") && arg.contains("port=1"))
        );
        assert!(args.iter().any(|arg| arg.contains("usb-host")
            && arg.contains("hostbus=1")
            && arg.contains("hostport=7.5.1")));
        assert!(args.iter().any(|arg| {
            arg.contains("usb-host")
                && arg.contains("vendorid=0x0451")
                && arg.contains("productid=0x16a0")
        }));
    }

    #[test]
    fn renders_virtio_vga_gl_when_gl_display_is_enabled() {
        let mut bus_register = BusRegister::new();
        let model = RuntimeModel::new(
            "gl-vm".to_string(),
            Cpu::new(CpuModel::Host, 2, 1, 1),
            Memory::megabytes(2048),
            Chipset::Q35(Q35Chipset::new(&mut bus_register)),
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            None,
            None,
            None,
            Some(DisplayModelBuilder::build(Display::Spice {
                spice: crate::runtime_model::Spice::new("127.0.0.1".to_string(), 5905, true)
                    .with_gl_enabled(true),
            })),
            None,
            None,
            bus_register,
        );

        let gpu: Arc<dyn PcieDeviceApi> = Arc::new(VirtioGpuController::default());
        model
            .register_pcie_device(0, gpu, Some(PcieAddress::new(1, 0)))
            .expect("virtio gpu registration should succeed");

        let args = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(args.iter().any(|arg| arg.contains("egl-headless,gl=core")));
        assert!(args.iter().any(|arg| arg.contains("virtio-vga-gl")));
        assert!(args.iter().any(|arg| arg.contains("max_hostmem=67108864")));
    }

    #[test]
    fn renders_lifecycle_and_qmp_monitoring_args_when_configured() {
        let mut bus_register = BusRegister::new();
        let model = RuntimeModel::new(
            "lifecycle-vm".to_string(),
            Cpu::new(CpuModel::Host, 2, 1, 1),
            Memory::megabytes(2048),
            Chipset::Q35(Q35Chipset::new(&mut bus_register)),
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            None,
            None,
            None,
            None,
            None,
            None,
            bus_register,
        )
        .with_lifecycle_config(Some(LifecycleConfig::new(
            Some("/run/qemu/lifecycle-vm.pid".to_string()),
            true,
            true,
            Some("/run/qemu/lifecycle-vm.qmp".to_string()),
            Some("/run/qemu/lifecycle-vm.event".to_string()),
        )));

        let args = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(args.iter().any(|arg| arg == "-pidfile"));
        assert!(args.iter().any(|arg| arg == "/run/qemu/lifecycle-vm.pid"));
        assert!(args.iter().any(|arg| arg == "-daemonize"));
        assert!(args.iter().any(|arg| arg == "-no-shutdown"));
        assert!(
            args.iter()
                .any(|arg| arg.contains("socket,id=qmp,path=/run/qemu/lifecycle-vm.qmp"))
        );
        assert!(args.iter().any(|arg| arg == "chardev=qmp,mode=control"));
        assert!(
            args.iter()
                .any(|arg| arg.contains("socket,id=qmp-event,path=/run/qemu/lifecycle-vm.event"))
        );
        assert!(
            args.iter()
                .any(|arg| arg == "chardev=qmp-event,mode=control")
        );
    }
}
