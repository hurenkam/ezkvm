use super::*;

#[test]
fn test_wakiza_matches_key_proxmox_fragments() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let config = VmConfig::from_file("input/felucia/108.yaml").unwrap();
    let has_cdrom = config
        .devices
        .drives
        .iter()
        .any(|drive| drive.r#type == "cdrom");
    let has_passthrough = config.host.pci.iter().any(|d| d.id.starts_with("hostpci0"));
    let has_usb = !config.host.usb.is_empty();
    let has_balloon = config
        .system
        .memory
        .ballooning
        .as_ref()
        .is_some_and(|balloon| balloon.enabled);
    let has_explicit_queue_sizes = config
        .devices
        .networks
        .iter()
        .any(|network| network.rx_queue_size.is_some() || network.tx_queue_size.is_some());
    let manager = QemuManager::new(config, CentralConfig::default());
    let args = manager.build_command().unwrap();
    let generated = format!(
        "{} {}",
        manager.binary_name(),
        args.iter()
            .map(|arg| arg.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );

    let proxmox_cmd = include_str!("../../input/felucia/108.qemu.cmd");

    let always_fragments: &[&[&str]] = &[
        &["if=pflash,unit=0", "readonly=on"],
        &["if=pflash,unit=1", "id=drive-efidisk0", "format=raw"],
        &[
            "qemu-xhci",
            "id=xhci",
            "p2=15",
            "p3=15",
            "bus=pci.1",
            "addr=0x1b",
        ],
        &["ich9-intel-hda", "id=audiodev0", "bus=pci.2", "addr=0xc"],
        &[
            "hda-micro",
            "id=audiodev0-codec0",
            "bus=audiodev0.0",
            "cad=0",
            "audiodev=spice-backend0",
        ],
        &[
            "hda-duplex",
            "id=audiodev0-codec1",
            "bus=audiodev0.0",
            "cad=1",
            "audiodev=spice-backend0",
        ],
        &["spice,id=spice-backend0"],
        &["pvscsi", "id=scsihw0", "bus=pci.0", "addr=0x5"],
        &[
            "scsi-hd",
            "drive=drive-scsi0",
            "id=scsi0",
            "bus=scsihw0.0",
            "scsi-id=0",
            "bootindex=100",
        ],
        &[
            "scsi-hd",
            "drive=drive-scsi1",
            "id=scsi1",
            "bus=scsihw0.0",
            "scsi-id=1",
        ],
        &[
            "virtio-net-pci",
            "netdev=net0",
            "mac=BC:24:11:3A:21:B7",
            "bus=pci.0",
            "addr=0x12",
            "bootindex=102",
        ],
        &["port=5903", "addr=0.0.0.0", "disable-ticketing=on"],
        &["virtio-mouse"],
        &["virtio-keyboard"],
        &["ivshmem-plain", "memdev=ivshmem0", "bus=pcie.0"],
        &[
            "memory-backend-file",
            "id=ivshmem0",
            "share=on",
            "mem-path=/dev/kvmfr0",
            "size=128M",
        ],
    ];

    for group in always_fragments {
        for fragment in *group {
            assert!(
                proxmox_cmd.contains(fragment),
                "Fixture is missing fragment: {fragment}"
            );
            assert!(
                generated.contains(fragment),
                "Generated command is missing fragment: {fragment}\n{generated}"
            );
        }
    }

    assert!(
        proxmox_cmd.contains("OVMF_CODE_4M.secboot.fd"),
        "Fixture is missing expected Proxmox secure-boot OVMF fragment"
    );
    let generated_ovmf_variants = ["OVMF_CODE_4M.secboot.fd", "OVMF_CODE.secboot.fd", "OVMF.fd"];
    assert!(
        generated_ovmf_variants
            .iter()
            .any(|fragment| generated.contains(fragment)),
        "Generated command is missing an accepted OVMF fragment ({:?})\n{}",
        generated_ovmf_variants,
        generated
    );

    if has_balloon {
        let balloon_fragments = [
            "virtio-balloon-pci",
            "id=balloon0",
            "bus=pci.0",
            "addr=0x3",
            "free-page-reporting=on",
        ];
        for fragment in balloon_fragments {
            assert!(
                proxmox_cmd.contains(fragment),
                "Fixture is missing fragment: {fragment}"
            );
            assert!(
                generated.contains(fragment),
                "Generated command is missing fragment: {fragment}\n{generated}"
            );
        }
    }

    if has_explicit_queue_sizes {
        let queue_fragments = ["rx_queue_size=1024", "tx_queue_size=256"];
        for fragment in queue_fragments {
            assert!(
                proxmox_cmd.contains(fragment),
                "Fixture is missing fragment: {fragment}"
            );
            assert!(
                generated.contains(fragment),
                "Generated command is missing fragment: {fragment}\n{generated}"
            );
        }
    }

    if has_passthrough {
        let passthrough_fragments: &[&[&str]] = &[
            &["-vga", "none", "-nographic"],
            &[
                "vfio-pci",
                "host=0000:03:00.0",
                "bus=ich9-pcie-port-1",
                "addr=0x0.0",
                "multifunction=on",
            ],
            &[
                "vfio-pci",
                "host=0000:03:00.1",
                "bus=ich9-pcie-port-1",
                "addr=0x0.1",
            ],
        ];
        for group in passthrough_fragments {
            for fragment in *group {
                assert!(
                    proxmox_cmd.contains(fragment),
                    "Fixture is missing fragment: {fragment}"
                );
                assert!(
                    generated.contains(fragment),
                    "Generated command is missing fragment: {fragment}\n{generated}"
                );
            }
        }
    }

    if has_usb {
        let usb_fragments = [
            "usb-host",
            "hostbus=1",
            "hostport=2.2",
            "id=usb0",
            "bus=xhci.0",
            "port=1",
        ];
        for fragment in usb_fragments {
            assert!(
                proxmox_cmd.contains(fragment),
                "Fixture is missing fragment: {fragment}"
            );
            assert!(
                generated.contains(fragment),
                "Generated command is missing fragment: {fragment}\n{generated}"
            );
        }
    }

    if has_cdrom {
        let cdrom_fragments = [
            "ide-cd",
            "drive=drive-ide2",
            "id=ide2",
            "bus=ide.1",
            "unit=0",
            "bootindex=101",
        ];
        for fragment in cdrom_fragments {
            assert!(
                proxmox_cmd.contains(fragment),
                "Fixture is missing fragment: {fragment}"
            );
            assert!(
                generated.contains(fragment),
                "Generated command is missing fragment: {fragment}\n{generated}"
            );
        }
    }

    let virtio_serial_controller_count = generated.matches("virtio-serial-pci").count();
    assert_eq!(
        virtio_serial_controller_count, 1,
        "Expected a single virtio-serial-pci controller in generated command\n{generated}"
    );
}
