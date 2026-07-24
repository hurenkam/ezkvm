use crate::config::qemu::bootindex;
use crate::config::qemu::handlers::storage;
use crate::config::qemu::{QemuCommandLineBuilder, QemuContext};
use crate::runtime::{
    Cdrom, Hdd, PcieAddress, PcieBusDeviceKind, PcieDevice, PvScsi, Runtime, Ssd,
    StorageDeviceType,
};

/// Dispatch over all 4 `PcieBusDeviceKind` variants a `Q35Chipset.pcie_bus` entry can hold.
///
/// `net_ordinal` is the caller-computed position of this entry among *only* the
/// `VirtioNet`-kind entries on `pcie_bus`, sorted by `PcieAddress` (RESEARCH.md's Critical
/// Gap formula: `N = ordinal position among VirtioNetPcie entries sorted by PcieAddress`).
/// It is meaningless for any other `PcieBusDeviceKind` and only read in the `VirtioNet` arm.
/// This is deliberately NOT the same as `address.device()` — `ProxmoxImporter` places NICs
/// starting at PCIe slot 20 (Plan 07-03) to avoid colliding with the pvscsi/hostpci slot
/// ranges, so `address.device()` for the felucia sample's single `net0` is `20`, not `0`;
/// using it directly as the boot-order label ordinal would never match `boot_order`'s
/// `"net0"` entry. `net_ordinal` decouples the PCIe bus *slot* (cosmetic/collision-avoidance)
/// from the Proxmox-style `net{N}` *label* (boot-order lookup key) — see Plan 07-04 Deviations.
pub(crate) fn emit_pcie_device(
    builder: &mut QemuCommandLineBuilder,
    _ctx: &QemuContext,
    runtime: &Runtime,
    address: &PcieAddress,
    device: &dyn PcieDevice,
    net_ordinal: u8,
) {
    match device.device_kind() {
        PcieBusDeviceKind::PvScsi => {
            let pvscsi = device.as_any().downcast_ref::<PvScsi>().unwrap();
            emit_pvscsi(builder, pvscsi, runtime.boot_order());
        }
        PcieBusDeviceKind::HostPci => {
            let host_pci = device
                .as_any()
                .downcast_ref::<crate::runtime::HostPci>()
                .unwrap();
            emit_hostpci(builder, address, host_pci);
        }
        PcieBusDeviceKind::Ivshmem => {
            let ivshmem = device
                .as_any()
                .downcast_ref::<crate::runtime::Ivshmem>()
                .unwrap();
            emit_ivshmem(builder, ivshmem);
        }
        PcieBusDeviceKind::VirtioNet => {
            let nic = device
                .as_any()
                .downcast_ref::<crate::runtime::VirtioNetPcie>()
                .unwrap();
            emit_virtio_net(builder, address, nic, net_ordinal, runtime.boot_order());
        }
    }
}

/// pvscsi controller line + sorted recursion into `scsi_bus`. `pub(crate)` so Plan 07-03's
/// `pci_bus` `PvScsi` arm can call it without duplicating logic. `boot_order` is forwarded
/// unmodified from the caller (root.rs's bus-walk loop, via `Runtime::boot_order()`) — this
/// function reconstructs each scsi entry's label itself (`bootindex::scsi_label`) and looks
/// up its bootindex (Plan 07-04, D-07).
pub(crate) fn emit_pvscsi(
    builder: &mut QemuCommandLineBuilder,
    pvscsi: &PvScsi,
    boot_order: &[String],
) {
    builder.push_device("-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5".to_string());

    let mut entries: Vec<_> = pvscsi.scsi_bus().iter().collect();
    entries.sort_by_key(|(a, _)| (*a.target(), *a.lun()));

    for (address, device) in entries {
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
        let bootindex = bootindex::lookup_bootindex(
            boot_order,
            &bootindex::scsi_label(*address.target()),
        );
        storage::emit_scsi_storage(builder, resource, device_type, *address.target(), bootindex);
    }
}

pub(crate) fn emit_hostpci(
    builder: &mut QemuCommandLineBuilder,
    address: &PcieAddress,
    host_pci: &crate::runtime::HostPci,
) {
    for function in host_pci.functions() {
        builder.push_device(format!(
            "-device vfio-pci,host={}.{},id=hostpci{}.{},bus=ich9-pcie-port-1,addr=0x{:x}.{}{}",
            host_pci.base_bdf(),
            function,
            address.device(),
            function,
            address.device(),
            function,
            if host_pci.functions().len() > 1 && *function == 0 {
                ",multifunction=on"
            } else {
                ""
            }
        ));
    }
}

pub(crate) fn emit_ivshmem(builder: &mut QemuCommandLineBuilder, ivshmem: &crate::runtime::Ivshmem) {
    builder.push_device(format!(
        "-device ivshmem-plain,memdev={},bus=pcie.0",
        ivshmem.id()
    ));
    builder.push_object(format!(
        "-object memory-backend-file,id={},share=on,mem-path={},size={}",
        ivshmem.id(),
        ivshmem.mem_path(),
        ivshmem.size()
    ));
}

pub(crate) fn emit_virtio_net(
    builder: &mut QemuCommandLineBuilder,
    address: &PcieAddress,
    nic: &crate::runtime::VirtioNetPcie,
    net_ordinal: u8,
    boot_order: &[String],
) {
    let net_id = format!("net{}", address.device());
    let bootindex =
        bootindex::lookup_bootindex(boot_order, &bootindex::net_label(net_ordinal));

    builder.push_netdev(format!(
        "-netdev type=tap,id={},vhost={}",
        net_id,
        if nic.vhost().unwrap_or(false) { "on" } else { "off" }
    ));

    builder.push_device(format!(
        "-device virtio-net-pci,mac={},netdev={},bus=pci.0,id={}{}{}{}",
        nic.mac_address().clone().unwrap_or_default(),
        net_id,
        net_id,
        nic.rx_queue_size()
            .map(|s| format!(",rx_queue_size={}", s))
            .unwrap_or_default(),
        nic.tx_queue_size()
            .map(|s| format!(",tx_queue_size={}", s))
            .unwrap_or_default(),
        bootindex
            .map(|b| format!(",bootindex={}", b))
            .unwrap_or_default()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::qemu::QemuContext;
    use crate::runtime::{HostPci, Ivshmem, PvScsiBuilder, ScsiAddress, VirtioNetPcie};
    use std::sync::Arc;

    fn make_ctx() -> QemuContext {
        QemuContext::new(
            "testvm".to_string(),
            "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
            None,
        )
    }

    // --- Task 1: storage.rs shared scsi emission + PvScsi controller + scsi_bus recursion ---

    #[test]
    fn test_07_02_pvscsi_controller_and_single_ssd_end_to_end() {
        let pvscsi = PvScsiBuilder::new()
            .with_scsi_device(
                Some(ScsiAddress::new(0, 0)),
                Arc::new(Ssd::new("/dev/vm1/vm-108-boot".to_string())),
            )
            .build();

        let mut builder = QemuCommandLineBuilder::new();
        emit_pvscsi(&mut builder, &pvscsi, &[]);
        let output = builder.build().to_string();

        assert!(output.contains("pvscsi,id=scsihw0"), "output was: {output}");
        assert!(output.contains("if=none,id=drive-scsi0"), "output was: {output}");
        assert!(output.contains("file=/dev/vm1/vm-108-boot"), "output was: {output}");
        assert!(output.contains("scsi-hd"), "output was: {output}");
        assert!(output.contains("drive=drive-scsi0"), "output was: {output}");
        assert!(output.contains("id=scsi0"), "output was: {output}");
    }

    #[test]
    fn test_07_02_pvscsi_cdrom_and_hdd_device_models() {
        let pvscsi = PvScsiBuilder::new()
            .with_scsi_device(
                Some(ScsiAddress::new(1, 0)),
                Arc::new(Cdrom::new("/dev/vm1/vm-108-cd".to_string())),
            )
            .with_scsi_device(
                Some(ScsiAddress::new(2, 0)),
                Arc::new(Hdd::new("/dev/vm1/vm-108-data".to_string())),
            )
            .build();

        let mut builder = QemuCommandLineBuilder::new();
        emit_pvscsi(&mut builder, &pvscsi, &[]);
        let output = builder.build().to_string();

        let cd_line = output
            .split("-device")
            .find(|s| s.contains("scsi-cd"))
            .expect("scsi-cd device line missing");
        assert!(cd_line.contains("scsi-cd"), "output was: {output}");

        let drive_cd = output
            .split("-drive")
            .find(|s| s.contains("drive-scsi1"))
            .expect("drive-scsi1 line missing");
        assert!(drive_cd.contains("media=cdrom"), "output was: {output}");

        let hd_line = output
            .split("-device")
            .find(|s| s.contains("id=scsi2"))
            .expect("scsi2 device line missing");
        assert!(hd_line.contains("scsi-hd"), "output was: {output}");
        assert!(!hd_line.contains("media="), "output was: {output}");
    }

    #[test]
    fn test_07_02_pvscsi_scsi_bus_sorted_by_target_not_insertion_order() {
        let pvscsi = PvScsiBuilder::new()
            .with_scsi_device(
                Some(ScsiAddress::new(1, 0)),
                Arc::new(Ssd::new("/dev/vm1/second".to_string())),
            )
            .with_scsi_device(
                Some(ScsiAddress::new(0, 0)),
                Arc::new(Ssd::new("/dev/vm1/first".to_string())),
            )
            .build();

        let mut builder = QemuCommandLineBuilder::new();
        emit_pvscsi(&mut builder, &pvscsi, &[]);
        let output = builder.build().to_string();

        let idx0 = output.find("drive-scsi0").expect("drive-scsi0 missing");
        let idx1 = output.find("drive-scsi1").expect("drive-scsi1 missing");
        assert!(idx0 < idx1, "output was: {output}");
    }

    // --- Task 2: HostPci multifunction vfio-pci emission ---

    #[test]
    fn test_07_02_hostpci_multifunction_flag_only_on_function_zero() {
        let host_pci = HostPci::new(
            "0000:03:00".to_string(),
            vec![0, 1],
            true,
            true,
            None,
            None,
        );
        let address = PcieAddress::new(0, 0);

        let mut builder = QemuCommandLineBuilder::new();
        emit_hostpci(&mut builder, &address, &host_pci);
        let cmdline = builder.build();
        let devices = cmdline.devices();

        assert_eq!(devices.len(), 2, "devices were: {:?}", devices);
        assert!(devices[0].contains("host=0000:03:00.0"), "devices[0] was: {}", devices[0]);
        assert!(devices[0].contains("id=hostpci0.0"), "devices[0] was: {}", devices[0]);
        assert!(devices[0].contains("multifunction=on"), "devices[0] was: {}", devices[0]);

        assert!(devices[1].contains("host=0000:03:00.1"), "devices[1] was: {}", devices[1]);
        assert!(devices[1].contains("id=hostpci0.1"), "devices[1] was: {}", devices[1]);
        assert!(!devices[1].contains("multifunction=on"), "devices[1] was: {}", devices[1]);
    }

    #[test]
    fn test_07_02_hostpci_single_function_no_multifunction_flag() {
        let host_pci = HostPci::new("0000:04:00".to_string(), vec![0], false, false, None, None);
        let address = PcieAddress::new(0, 0);

        let mut builder = QemuCommandLineBuilder::new();
        emit_hostpci(&mut builder, &address, &host_pci);
        let cmdline = builder.build();
        let devices = cmdline.devices();

        assert_eq!(devices.len(), 1, "devices were: {:?}", devices);
        assert!(devices[0].contains("host=0000:04:00.0"), "devices[0] was: {}", devices[0]);
        assert!(devices[0].contains("id=hostpci0.0"), "devices[0] was: {}", devices[0]);
        assert!(!devices[0].contains("multifunction=on"), "devices[0] was: {}", devices[0]);
    }

    // --- Task 3: Ivshmem object+device pair, VirtioNet netdev+device pair, root.rs wiring ---

    #[test]
    fn test_07_02_ivshmem_object_before_device_in_display_order() {
        let ivshmem = Ivshmem::new(
            "ivshmem0".to_string(),
            "/dev/kvmfr0".to_string(),
            "128M".to_string(),
        );

        let mut builder = QemuCommandLineBuilder::new();
        emit_ivshmem(&mut builder, &ivshmem);
        let cmdline = builder.build();

        assert!(cmdline
            .devices()
            .iter()
            .any(|d| d.contains("ivshmem-plain,memdev=ivshmem0")));
        assert!(cmdline.objects().iter().any(|o| o.contains("memory-backend-file,id=ivshmem0")
            && o.contains("mem-path=/dev/kvmfr0")
            && o.contains("size=128M")));

        let output = cmdline.to_string();
        let idx_object = output.find("memory-backend-file").expect("object missing");
        let idx_device = output.find("ivshmem-plain").expect("device missing");
        assert!(idx_object < idx_device, "output was: {output}");
    }

    #[test]
    fn test_07_02_virtio_net_netdev_before_device_in_display_order() {
        let nic = VirtioNetPcie::new(
            Some("vmbr0".to_string()),
            Some("BC:24:11:3A:21:B7".to_string()),
            Some(1024),
            Some(256),
            Some(true),
        );
        let address = PcieAddress::new(0, 0);

        let mut builder = QemuCommandLineBuilder::new();
        emit_virtio_net(&mut builder, &address, &nic, 0, &[]);
        let cmdline = builder.build();

        assert!(cmdline.netdevs().iter().any(|n| n.contains("type=tap,id=net0")));
        assert!(cmdline.devices().iter().any(|d| d.contains("virtio-net-pci")
            && d.contains("mac=BC:24:11:3A:21:B7")
            && d.contains("netdev=net0")));

        let output = cmdline.to_string();
        let idx_netdev = output.find("id=net0").expect("netdev missing");
        let idx_device = output.find("netdev=net0").expect("device missing");
        assert!(idx_netdev < idx_device, "output was: {output}");
    }

    #[test]
    fn test_07_02_root_chipset_walks_pcie_bus_for_pvscsi_and_hostpci() {
        use crate::runtime::{Chipset, Memory, Q35ChipsetBuilder, Runtime, RuntimeBuilder};

        let pvscsi = PvScsiBuilder::new()
            .with_scsi_device(
                Some(ScsiAddress::new(0, 0)),
                Arc::new(Ssd::new("/dev/vm1/vm-108-boot".to_string())),
            )
            .build();
        let host_pci = HostPci::new("0000:03:00".to_string(), vec![0], true, true, None, None);

        let runtime: Runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(16384))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_pcie_device(Some(PcieAddress::new(16, 0)), Arc::new(pvscsi))
                    .with_host_pci(0, Arc::new(host_pci))
                    .build(),
            ))
            .build()
            .unwrap();

        let cmdline = crate::config::qemu::QemuCommandLine::try_from((runtime, make_ctx())).unwrap();
        let output = cmdline.to_string();

        assert!(output.contains("pvscsi,id=scsihw0"), "output was: {output}");
        assert!(output.contains("hostpci0.0"), "output was: {output}");
    }
}
