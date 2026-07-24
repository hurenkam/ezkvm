use crate::config::qemu::bootindex;
use crate::config::qemu::handlers;
use crate::config::qemu::{QemuCommandLineBuilder, QemuContext, QemuConversionError};
use crate::runtime::{
    AudioDevice, Cdrom, Chipset, CpuTopology, EfiDisk, Hdd, Memory, RawArgs, RootDevice,
    RootDeviceKind, Runtime, Ssd, SpiceDisplay, StorageDeviceType, TpmState, VgaConfig,
};

pub(crate) fn emit_root_device(
    builder: &mut QemuCommandLineBuilder,
    ctx: &QemuContext,
    runtime: &Runtime,
    device: &dyn RootDevice,
) -> Result<(), QemuConversionError> {
    match device.device_kind() {
        RootDeviceKind::Memory => {
            let memory = device.as_any().downcast_ref::<Memory>().unwrap();
            builder.push_machine(format!("-m {}", memory.size()));
        }
        RootDeviceKind::Chipset => {
            let chipset = device.as_any().downcast_ref::<Chipset>().unwrap();
            match chipset {
                Chipset::Q35(_) => builder.push_machine("-machine q35"),
                Chipset::I440FX => builder.push_machine("-machine pc"),
            }
            if let Chipset::Q35(q35) = chipset {
                let mut pcie_entries: Vec<_> = q35.pcie_bus().iter().collect();
                pcie_entries.sort_by_key(|(a, _)| (*a.device(), *a.function()));
                let mut net_ordinal: u8 = 0;
                for (address, device) in pcie_entries {
                    let this_net_ordinal = net_ordinal;
                    if device.device_kind() == crate::runtime::PcieBusDeviceKind::VirtioNet {
                        net_ordinal += 1;
                    }
                    handlers::pcie::emit_pcie_device(
                        builder,
                        ctx,
                        runtime,
                        address,
                        device.as_ref(),
                        this_net_ordinal,
                    );
                }

                let mut pci_entries: Vec<_> = q35.pci_bus().iter().collect();
                pci_entries.sort_by_key(|(a, _)| (*a.device(), *a.function()));
                for (_address, device) in pci_entries {
                    handlers::pci::emit_pci_device(builder, device.as_ref(), runtime.boot_order());
                }

                let mut sata_entries: Vec<_> = q35.sata_bus().iter().collect();
                sata_entries.sort_by_key(|(a, _)| (*a.port(), *a.device()));
                for (address, device) in sata_entries {
                    let device_type = device.storage_options().device_type;
                    let resource: &str = match device_type {
                        StorageDeviceType::Ssd => {
                            &device.as_any().downcast_ref::<Ssd>().unwrap().resource
                        }
                        StorageDeviceType::Hdd => {
                            &device.as_any().downcast_ref::<Hdd>().unwrap().resource
                        }
                        StorageDeviceType::Odd => {
                            &device.as_any().downcast_ref::<Cdrom>().unwrap().resource
                        }
                    };
                    handlers::storage::emit_sata_storage(
                        builder,
                        resource,
                        device_type,
                        *address.port(),
                        bootindex::lookup_bootindex(
                            runtime.boot_order(),
                            &bootindex::sata_label(*address.port()),
                        ),
                    );
                }

                let mut ide_entries: Vec<_> = q35.ide_bus().iter().collect();
                ide_entries.sort_by_key(|(a, _)| (*a.channel(), *a.device()));
                for (address, device) in ide_entries {
                    let device_type = device.storage_options().device_type;
                    let resource: &str = match device_type {
                        StorageDeviceType::Ssd => {
                            &device.as_any().downcast_ref::<Ssd>().unwrap().resource
                        }
                        StorageDeviceType::Hdd => {
                            &device.as_any().downcast_ref::<Hdd>().unwrap().resource
                        }
                        StorageDeviceType::Odd => {
                            &device.as_any().downcast_ref::<Cdrom>().unwrap().resource
                        }
                    };
                    handlers::storage::emit_ide_storage(
                        builder,
                        resource,
                        device_type,
                        *address.channel(),
                        *address.device(),
                        bootindex::lookup_bootindex(
                            runtime.boot_order(),
                            &bootindex::ide_label(*address.channel(), *address.device()),
                        ),
                    );
                }

                let mut usb_entries: Vec<_> = q35.usb_bus().iter().collect();
                usb_entries.sort_by_key(|(a, _)| a.port().clone());
                for (address, device) in usb_entries {
                    handlers::usb::emit_usb_device(builder, address, device.as_ref());
                }
            }
        }
        RootDeviceKind::EfiDisk => {
            let efidisk = device.as_any().downcast_ref::<EfiDisk>().unwrap();
            builder.push_firmware(format!(
                "-drive if=pflash,unit=0,format=raw,readonly=on,file={}",
                ctx.ovmf_code_path()
            ));
            builder.push_firmware(format!(
                "-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file={},size={}",
                efidisk.storage_volume(),
                efidisk
                    .block_device_size_bytes()
                    .expect("EfiDisk.block_device_size_bytes required for pflash size= (D-06)")
            ));
        }
        RootDeviceKind::TpmState => {
            let _tpm = device.as_any().downcast_ref::<TpmState>().unwrap();
            let socket_path = ctx
                .tpm_socket_path()
                .as_ref()
                .expect("QemuContext.tpm_socket_path required when TpmState present (D-06)");
            builder.push_chardev(format!("-chardev socket,id=tpmchar,path={}", socket_path));
            builder.push_tpm("-tpmdev emulator,id=tpmdev,chardev=tpmchar".to_string());
            builder.push_device("-device tpm-tis,tpmdev=tpmdev".to_string());
        }
        RootDeviceKind::AudioDevice => {
            let audio = device.as_any().downcast_ref::<AudioDevice>().unwrap();
            builder.push_device(format!(
                "-device {},id=audiodev0,bus=pci.2,addr=0xc",
                audio.device_type()
            ));
            if audio.driver() == "spice" {
                builder.push_device(
                    "-device hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0"
                        .to_string(),
                );
                builder.push_device(
                    "-device hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0"
                        .to_string(),
                );
            }
            builder.push_misc(format!("-audiodev {},id=spice-backend0", audio.driver()));
        }
        RootDeviceKind::SpiceDisplay => {
            let spice = device.as_any().downcast_ref::<SpiceDisplay>().unwrap();
            let mut parts: Vec<String> = Vec::new();
            if let Some(port) = spice.port() {
                parts.push(format!("port={}", port));
            }
            if let Some(addr) = spice.addr() {
                parts.push(format!("addr={}", addr));
            }
            if *spice.disable_ticketing() {
                parts.push("disable-ticketing=on".to_string());
            }
            if *spice.gl() {
                parts.push("gl=on".to_string());
            }
            if let Some(rendernode) = spice.rendernode() {
                parts.push(format!("rendernode={}", rendernode));
            }
            builder.push_misc(format!("-spice {}", parts.join(",")));
        }
        RootDeviceKind::RawArgs => {
            let raw_args = device.as_any().downcast_ref::<RawArgs>().unwrap();
            builder.push_misc(raw_args.0.clone());
        }
        RootDeviceKind::CpuTopology => {
            let cpu = device.as_any().downcast_ref::<CpuTopology>().unwrap();
            let total_cpus = (*cpu.sockets() as u32) * (*cpu.cores() as u32);
            builder.push_machine(format!(
                "-smp {},sockets={},cores={},maxcpus={}",
                total_cpus,
                cpu.sockets(),
                cpu.cores(),
                total_cpus
            ));
            builder.push_machine(format!("-cpu {}", cpu.cpu_type()));
        }
        RootDeviceKind::VgaConfig => {
            let vga = device.as_any().downcast_ref::<VgaConfig>().unwrap();
            if vga.mode() == "none" {
                builder.push_misc("-vga none".to_string());
                builder.push_misc("-nographic".to_string());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
    use crate::config::qemu::QemuCommandLine;
    use crate::runtime::{
        IdeAddress, PcieAddress, PvScsiBuilder, Q35ChipsetBuilder, RuntimeBuilder, SataAddress,
        ScsiAddress, Ssd, VirtioNetPcie,
    };
    use std::str::FromStr;
    use std::sync::Arc;

    fn make_ctx() -> QemuContext {
        QemuContext::new(
            "testvm".to_string(),
            "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
            None,
        )
    }

    /// Imports the felucia `108.conf` fixture and returns just its `Chipset` + `boot_order`,
    /// rebuilt into a fresh minimal `Runtime`. This deliberately does NOT reuse the imported
    /// `Runtime`'s `EfiDisk`/`TpmState` root devices: `ProxmoxImporter` leaves
    /// `EfiDisk.block_device_size_bytes` as `None` for felucia (a pre-existing, unrelated gap —
    /// not introduced by this plan, out of this plan's scope per the executor's scope
    /// boundary), which makes `emit_root_device`'s `EfiDisk` arm `.expect()`-panic. The bus
    /// maps this plan actually exercises (`pcie_bus`/`ide_bus`/`sata_bus`) and `boot_order` are
    /// unaffected by that gap, so isolating them here proves the real Proxmox-parsed data flows
    /// through bootindex reconstruction end-to-end without being blocked by an unrelated bug.
    fn felucia_chipset_and_boot_order() -> (Chipset, Vec<String>) {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let conf_str = std::fs::read_to_string(format!("{manifest}/input/felucia/108.conf"))
            .expect("108.conf not found");
        let storage_str =
            std::fs::read_to_string(format!("{manifest}/input/felucia/storage.cfg"))
                .expect("storage.cfg not found");
        let vm_conf = ProxmoxVmConf::from_str(&conf_str).expect("parse 108.conf");
        let storage_conf = ProxmoxStorageConf::from_str(&storage_str).expect("parse storage.cfg");
        let full_runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
            .into_runtime()
            .expect("into_runtime");
        let chipset = full_runtime
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::Chipset)
            .expect("Chipset root device missing")
            .as_any()
            .downcast_ref::<Chipset>()
            .expect("downcast to Chipset")
            .clone();
        (chipset, full_runtime.boot_order().clone())
    }

    // --- Task 2: felucia end-to-end bootindex proof (scsi0/ide2/net0 -> 100/101/102) ---

    #[test]
    fn test_07_04_felucia_scsi0_ide2_net0_bootindex_100_101_102() {
        let (chipset, boot_order) = felucia_chipset_and_boot_order();
        assert_eq!(
            boot_order,
            vec!["scsi0".to_string(), "ide2".to_string(), "net0".to_string()]
        );

        let runtime = RuntimeBuilder::new()
            .with_chipset(chipset)
            .with_boot_order(boot_order)
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, make_ctx())).unwrap();
        let output = cmdline.to_string();

        let scsi0_line = output
            .split("-device")
            .find(|s| s.contains("id=scsi0"))
            .expect("scsi0 device line missing");
        assert!(scsi0_line.contains("bootindex=100"), "line was: {scsi0_line}");

        let ide2_line = output
            .split("-device")
            .find(|s| s.contains("id=ide2"))
            .expect("ide2 device line missing");
        assert!(ide2_line.contains("bootindex=101"), "line was: {ide2_line}");

        let net0_line = output
            .split("-device")
            .find(|s| s.contains("virtio-net-pci"))
            .expect("net0 device line missing");
        assert!(net0_line.contains("bootindex=102"), "line was: {net0_line}");
    }

    #[test]
    fn test_07_04_felucia_sata_absent_from_boot_order_gets_no_bootindex() {
        // felucia's 108.conf has no sata disks, so a synthetic sata Runtime is used here
        // (per the plan's explicit fallback for this test) to prove the "absent from
        // boot_order -> no bootindex token" rule on a realistic (non-scsi/ide/net) device.
        let q35 = Q35ChipsetBuilder::new()
            .with_sata_device(
                Some(SataAddress::new(0, 0)),
                Arc::new(Ssd::new("/dev/vm1/extra-sata".to_string())),
            )
            .build();
        let runtime = RuntimeBuilder::new()
            .with_chipset(Chipset::Q35(q35))
            .with_boot_order(vec!["scsi0".to_string(), "ide2".to_string(), "net0".to_string()])
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, make_ctx())).unwrap();
        let output = cmdline.to_string();

        assert!(output.contains("id=sata0"), "output was: {output}");
        let sata0_line = output
            .split("-device")
            .find(|s| s.contains("id=sata0"))
            .expect("sata0 device line missing");
        assert!(!sata0_line.contains("bootindex="), "line was: {sata0_line}");
    }

    // --- Task 3: multi-NIC ordinal behavior + empty-boot_order no-op (Assumption A1) ---

    #[test]
    fn test_07_04_two_nic_ordinal_is_position_among_virtio_net_entries_not_raw_pcie_slot() {
        // Deviation from the plan's literal Task 3 text (documented in SUMMARY.md): net_label's
        // ordinal is the position among VirtioNetPcie entries sorted by PcieAddress (RESEARCH.md's
        // documented formula), NOT the raw PcieAddress.device() slot number. ProxmoxImporter
        // places net devices starting at pcie slot 20 (Plan 07-03) specifically to avoid bus-map
        // key collisions with pvscsi/hostpci — that slot number is a bus-address collision-avoidance
        // detail, not the Proxmox-visible net{N} label. Using it directly as the label ordinal
        // would make the felucia end-to-end proof above (net0 -> bootindex=102) unsatisfiable,
        // since felucia's single NIC sits at slot 20, not 0. This test documents the corrected,
        // must_haves-compliant behavior: two NICs at slots 20 and 21 are labeled "net0"/"net1" by
        // sorted position, not "net20"/"net21".
        let nic0 = VirtioNetPcie::new(
            Some("vmbr0".to_string()),
            Some("BC:24:11:3A:21:B7".to_string()),
            None,
            None,
            Some(true),
        );
        let nic1 = VirtioNetPcie::new(
            Some("vmbr0".to_string()),
            Some("BC:24:11:3A:21:B8".to_string()),
            None,
            None,
            Some(true),
        );
        let q35 = Q35ChipsetBuilder::new()
            .with_pcie_device(Some(PcieAddress::new(20, 0)), Arc::new(nic0))
            .with_pcie_device(Some(PcieAddress::new(21, 0)), Arc::new(nic1))
            .build();
        let runtime = RuntimeBuilder::new()
            .with_chipset(Chipset::Q35(q35))
            .with_boot_order(vec!["net1".to_string()])
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, make_ctx())).unwrap();
        let output = cmdline.to_string();

        assert_eq!(output.matches("bootindex=").count(), 1, "output was: {output}");

        let mac_b8_line = output
            .split("-device")
            .find(|s| s.contains("BC:24:11:3A:21:B8"))
            .expect("second NIC's device line missing");
        assert!(mac_b8_line.contains("bootindex=100"), "line was: {mac_b8_line}");

        let mac_b7_line = output
            .split("-device")
            .find(|s| s.contains("BC:24:11:3A:21:B7"))
            .expect("first NIC's device line missing");
        assert!(!mac_b7_line.contains("bootindex="), "line was: {mac_b7_line}");
    }

    #[test]
    fn test_07_04_empty_boot_order_emits_zero_bootindex_tokens() {
        let pvscsi = PvScsiBuilder::new()
            .with_scsi_device(
                Some(ScsiAddress::new(0, 0)),
                Arc::new(Ssd::new("/dev/vm1/boot".to_string())),
            )
            .build();
        let q35 = Q35ChipsetBuilder::new()
            .with_pcie_device(Some(PcieAddress::new(16, 0)), Arc::new(pvscsi))
            .with_sata_device(
                Some(SataAddress::new(0, 0)),
                Arc::new(Ssd::new("/dev/vm1/extra".to_string())),
            )
            .with_ide_device(
                Some(IdeAddress::new(1, 0)),
                Arc::new(crate::runtime::Cdrom::new("none".to_string())),
            )
            .build();
        let runtime = RuntimeBuilder::new()
            .with_chipset(Chipset::Q35(q35))
            .build()
            .unwrap();
        assert!(runtime.boot_order().is_empty());

        let cmdline = QemuCommandLine::try_from((runtime, make_ctx())).unwrap();
        let output = cmdline.to_string();

        assert_eq!(output.matches("bootindex=").count(), 0, "output was: {output}");
    }
}
