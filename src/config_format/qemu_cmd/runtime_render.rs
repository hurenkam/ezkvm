use crate::runtime_model::{
    Ac97Controller, AudioBackend, AudioController, BiosModel, Chipset, Cpu, CpuModel, Display,
    Ich9IntelHdaController, IvshmemPlainController, Memory, NetworkResource,
    PassthroughGpuController, PassthroughPcieController, PcieAddress, PcieDeviceApi,
    PvScsiController, QxlGpuController, RuntimeModel, ScsiAddress, ScsiControllerApi,
    StandardGpuController, StorageDeviceKind, StorageResource, VirtioGpuController,
    VirtioNetController,
};

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
            let listen = if vnc.listen().is_empty() {
                "0.0.0.0"
            } else {
                vnc.listen()
            };
            vec!["-vnc".to_string(), format!("{}:{}", listen, vnc.port())]
        }
        Display::Spice { spice } => {
            let listen = if spice.listen().is_empty() {
                "0.0.0.0"
            } else {
                spice.listen()
            };
            let mut spec = format!("port={},addr={}", spice.port(), listen);
            if *spice.disable_ticketing() {
                spec.push_str(",disable-ticketing=on");
            }
            vec!["-spice".to_string(), spec]
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

fn render_bus_args(runtime: &RuntimeModel) -> Vec<String> {
    let mut args = Vec::new();

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
                    args.extend(render_pcie_device(bus_id, address.clone(), device.as_ref()));
                }
            }
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

fn render_pcie_device(bus: u8, address: PcieAddress, device: &dyn PcieDeviceApi) -> Vec<String> {
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

fn render_ide_drive_args(
    resource: &StorageResource,
    bus: u8,
    unit: u8,
    kind: StorageDeviceKind,
) -> Vec<String> {
    let drive_id = format!("drive-ide{unit}");
    let device_id = format!("ide{unit}");
    let mut drive_options = vec!["if=none".to_string(), format!("id={drive_id}")];

    match resource {
        StorageResource::File { file } => drive_options.push(format!("file={file}")),
        StorageResource::BlockDevice { block_device } => {
            drive_options.push(format!("file={block_device}"))
        }
    }

    drive_options.push("format=raw".to_string());

    let device_type = match kind {
        StorageDeviceKind::Cdrom => {
            drive_options.push("media=cdrom".to_string());
            drive_options.push("readonly=on".to_string());
            "ide-cd"
        }
        StorageDeviceKind::Hdd | StorageDeviceKind::Ssd => "ide-hd",
    };

    vec![
        "-drive".to_string(),
        drive_options.join(","),
        "-device".to_string(),
        format!("{device_type},bus=ide.{bus},unit={unit},drive={drive_id},id={device_id}"),
    ]
}

fn render_sata_drive_args(resource: &StorageResource, address: u8) -> Vec<String> {
    let drive_id = format!("drive-sata{address}");
    let device_id = format!("sata{address}");
    let mut drive_options = vec![format!("id={drive_id}")];

    match resource {
        StorageResource::File { file } => drive_options.push(format!("file={file}")),
        StorageResource::BlockDevice { block_device } => {
            drive_options.push(format!("file={block_device}"))
        }
    }

    drive_options.push("if=none".to_string());
    drive_options.push("format=raw".to_string());
    drive_options.push("discard=unmap".to_string());
    drive_options.push("detect-zeroes=unmap".to_string());

    vec![
        "-drive".to_string(),
        drive_options.join(","),
        "-device".to_string(),
        format!("ide-hd,id={device_id},drive={drive_id},bus=ahci0.{address}"),
    ]
}

fn render_scsi_drive_args(
    resource: &StorageResource,
    bus: u8,
    address: ScsiAddress,
    kind: StorageDeviceKind,
) -> Vec<String> {
    let drive_id = format!("drive-scsi{}", address.lun);
    let device_id = format!("scsi{}", address.lun);
    let mut drive_options = vec![format!("id={drive_id}")];

    match resource {
        StorageResource::File { file } => drive_options.push(format!("file={file}")),
        StorageResource::BlockDevice { block_device } => {
            drive_options.push(format!("file={block_device}"))
        }
    }

    drive_options.push("if=none".to_string());
    drive_options.push("format=raw".to_string());
    drive_options.push("discard=unmap".to_string());
    drive_options.push("detect-zeroes=unmap".to_string());

    let device_type = match kind {
        StorageDeviceKind::Cdrom => {
            drive_options.push("media=cdrom".to_string());
            drive_options.push("readonly=on".to_string());
            "scsi-cd"
        }
        StorageDeviceKind::Hdd | StorageDeviceKind::Ssd => "scsi-hd",
    };

    vec![
        "-drive".to_string(),
        drive_options.join(","),
        "-device".to_string(),
        format!(
            "{device_type},bus=scsihw{bus}.0,channel=0,scsi-id={},lun={},drive={drive_id},id={device_id}",
            address.target, address.lun
        ),
    ]
}

fn format_pcie_addr(address: &PcieAddress) -> String {
    if address.function() == 0 {
        format!("0x{:x}", address.device())
    } else {
        format!("0x{:x}.{}", address.device(), address.function())
    }
}
