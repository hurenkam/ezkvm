// Phase 4 Plan 04-01-B through 04-03-B: ProxmoxImporter

use std::sync::Arc;

use crate::config::proxmox::error::ProxmoxImportError;
use crate::config::proxmox::storage::StorageResolver;
use crate::config::proxmox::{ProxmoxDiskConf, ProxmoxStorageConf, ProxmoxVmConf};
use crate::runtime::{
    AudioDevice, Cdrom, Chipset, CpuTopology, EfiDisk, GenericUsbDevice, Hdd, HostPci, IdeAddress,
    IdeDevice, Memory, PcieAddress, PvScsiBuilder, Q35ChipsetBuilder, RawArgs, RuntimeBuilder,
    SataAddress, SataDevice, ScsiAddress, ScsiDevice, Ssd, TpmState, UsbAddress, UsbDeviceKind,
    VgaConfig, VirtioNetPcie,
};

pub struct ProxmoxImporter {
    pub vm_conf: ProxmoxVmConf,
    pub storage_conf: ProxmoxStorageConf,
    pub vmid: u32,
}

impl ProxmoxImporter {
    pub fn new(vm_conf: ProxmoxVmConf, storage_conf: ProxmoxStorageConf, vmid: u32) -> Self {
        ProxmoxImporter { vm_conf, storage_conf, vmid }
    }

    pub fn into_runtime(self) -> Result<crate::runtime::Runtime, ProxmoxImportError> {
        let memory_mb = self.vm_conf.memory.ok_or(ProxmoxImportError::MissingMemory)?;

        if let Some(m) = &self.vm_conf.machine {
            if !m.contains("q35") {
                return Err(ProxmoxImportError::UnsupportedMachine { machine: m.clone() });
            }
        }

        let mut chipset_builder = Q35ChipsetBuilder::new();

        let resolver = StorageResolver::new(&self.storage_conf, self.vmid);

        // Scsi disks → PvScsi on PCIe slot 16
        let mut pvscsi_builder = PvScsiBuilder::new();
        for (idx, disk_conf) in &self.vm_conf.scsi {
            if !disk_conf.volume.contains(':') { continue; }
            let resolved = resolver.resolve(&disk_conf.volume)?;
            let device: Arc<dyn ScsiDevice> = classify_disk_scsi_resolved(disk_conf, resolved);
            pvscsi_builder = pvscsi_builder
                .with_scsi_device(Some(ScsiAddress::new(*idx, 0)), device);
        }
        chipset_builder = chipset_builder
            .with_pcie_device(Some(PcieAddress::new(16, 0)), Arc::new(pvscsi_builder.build()));

        // Sata disks
        for (idx, disk_conf) in &self.vm_conf.sata {
            if !disk_conf.volume.contains(':') { continue; }
            let resolved = resolver.resolve(&disk_conf.volume)?;
            let device: Arc<dyn SataDevice> = classify_disk_sata_resolved(disk_conf, resolved);
            chipset_builder = chipset_builder
                .with_sata_device(Some(SataAddress::new(*idx, 0)), device);
        }

        // Ide disks. A Proxmox `ideN: none,media=cdrom` entry (no backing volume — an empty
        // CD-ROM drive) is a valid, common configuration (confirmed by the felucia fixture's
        // `ide2: none,media=cdrom`) and must still occupy its `ide_bus` slot so downstream
        // bootindex reconstruction (Plan 07-04) can find it; it is not merely skippable like
        // an unresolvable volume string would be for other bus types.
        for (idx, disk_conf) in &self.vm_conf.ide {
            let device: Arc<dyn IdeDevice> = if disk_conf.volume == "none" {
                Arc::new(Cdrom::new("none".to_string()))
            } else {
                if !disk_conf.volume.contains(':') { continue; }
                let resolved = resolver.resolve(&disk_conf.volume)?;
                classify_disk_ide_resolved(disk_conf, resolved)
            };
            // Proxmox's `ideN` index addresses a single (channel, device) slot on a 2-channel,
            // 2-device-per-channel IDE bus: channel = N / 2, device = N % 2 (confirmed by the
            // felucia sample: `ide2` -> `bus=ide.1,unit=0` -> channel=1, device=0). The
            // previous `IdeAddress::new(*idx, 0)` used the raw Proxmox index as `channel`
            // directly, which is only correct for N < 2 — see Plan 07-04 Deviations.
            chipset_builder = chipset_builder
                .with_ide_device(Some(IdeAddress::new(*idx / 2, *idx % 2)), device);
        }

        // Virtio disks → sata bus at port 32+idx
        for (idx, disk_conf) in &self.vm_conf.virtio {
            if !disk_conf.volume.contains(':') { continue; }
            let resolved = resolver.resolve(&disk_conf.volume)?;
            let device: Arc<dyn SataDevice> = classify_disk_sata_resolved(disk_conf, resolved);
            chipset_builder = chipset_builder
                .with_sata_device(Some(SataAddress::new(32 + idx, 0)), device);
        }

        // HostPci — must be added before chipset_builder.build()
        for (idx, pci_conf) in &self.vm_conf.hostpci {
            let raw_bdf = &pci_conf.bdf;
            // Strip .N function suffix if present (e.g. "0000:03:00.0" → "0000:03:00")
            let base_bdf = if let Some(pos) = raw_bdf.rfind('.') {
                raw_bdf[..pos].to_string()
            } else {
                raw_bdf.clone()
            };
            let pcie = pci_conf.options.get("pcie").map(|v| v == "1").unwrap_or(false);
            let x_vga = pci_conf.options.get("x-vga").map(|v| v == "1").unwrap_or(false);
            let functions = if x_vga { vec![0u8, 1u8] } else { vec![0u8] };
            let rombar = pci_conf.options.get("rombar").map(|v| v == "1");
            let romfile = pci_conf.options.get("romfile").cloned();
            let host_pci = HostPci::new(base_bdf, functions, pcie, x_vga, rombar, romfile);
            chipset_builder = chipset_builder.with_host_pci(*idx, Arc::new(host_pci));
        }

        // Net devices (D-01 import-side completion): net devices start at pcie slot 20 to
        // avoid colliding with scsi (16) and hostpci (0-N) slots
        for (idx, net_conf) in &self.vm_conf.net {
            let resource = net_conf.options.get("bridge").cloned();
            let nic = VirtioNetPcie::new(resource, Some(net_conf.mac.clone()), None, None, Some(true));
            chipset_builder = chipset_builder
                .with_pcie_device(Some(PcieAddress::new(20 + idx, 0)), Arc::new(nic));
        }

        // USB devices (D-01 import-side completion)
        for (idx, usb_conf) in &self.vm_conf.usb {
            let generic = GenericUsbDevice::new(UsbDeviceKind::HostPassthrough {
                resource: usb_conf.host.clone(),
            });
            chipset_builder = chipset_builder
                .with_usb_device(Some(UsbAddress::new(idx.to_string())), Arc::new(generic));
        }

        let mut builder = RuntimeBuilder::new()
            .with_memory(Memory::new(memory_mb as usize))
            .with_chipset(Chipset::Q35(chipset_builder.build()));

        // EfiDisk
        if let Some(efi) = &self.vm_conf.efidisk {
            let resolved = resolver.resolve(&efi.volume)?;
            let efitype = efi.options.get("efitype").cloned();
            let pre_enrolled_keys = efi.options.get("pre-enrolled-keys")
                .map(|v| v == "1").unwrap_or(false);
            let logical_size = efi.options.get("size").cloned().unwrap_or_default();
            let efidisk = EfiDisk::new(
                resolved,
                efitype,
                pre_enrolled_keys,
                None,
                logical_size,
                None,
            );
            builder = builder.with_efidisk(efidisk);
        }

        // TpmState
        if let Some(tpm) = &self.vm_conf.tpmstate {
            let resolved = resolver.resolve(&tpm.volume)?;
            builder = builder.with_tpmstate(TpmState::new(resolved, "v2.0".to_string()));
        }

        // AudioDevice
        if let Some(audio) = &self.vm_conf.audio {
            let device_type = audio.device.clone().unwrap_or_default();
            let driver = audio.driver.clone().unwrap_or_default();
            builder = builder.with_audio_device(AudioDevice::new(device_type, driver));
        }

        // RawArgs (verbatim — never tokenize)
        if let Some(args) = &self.vm_conf.args {
            builder = builder.with_raw_args(RawArgs(args.clone()));
        }

        // Boot order (D-07): "order=a;b;c" -> ["a", "b", "c"]
        if let Some(raw) = &self.vm_conf.boot {
            let order_str = raw.strip_prefix("order=").unwrap_or(raw);
            let order_list: Vec<String> = order_str.split(';').map(str::to_string).collect();
            builder = builder.with_boot_order(order_list);
        }

        // CPU topology (-smp/-cpu)
        if self.vm_conf.cores.is_some()
            || self.vm_conf.sockets.is_some()
            || self.vm_conf.cpu.is_some()
        {
            let cpu_topology = CpuTopology::new(
                self.vm_conf.sockets.unwrap_or(1),
                self.vm_conf.cores.unwrap_or(1),
                self.vm_conf.cpu.clone().unwrap_or_else(|| "qemu64".to_string()),
            );
            builder = builder.with_cpu_topology(cpu_topology);
        }

        // vga: none flag
        if let Some(mode) = &self.vm_conf.vga {
            builder = builder.with_vga_config(VgaConfig::new(mode.clone()));
        }

        let runtime = builder.build()
            .map_err(|_| unreachable!("RuntimeBuilder::build() never fails"))?;
        Ok(runtime)
    }
}

fn is_cdrom(disk_conf: &ProxmoxDiskConf) -> bool {
    disk_conf.options.get("media").map(|v| v == "cdrom").unwrap_or(false)
}

fn is_ssd(disk_conf: &ProxmoxDiskConf) -> bool {
    disk_conf.options.get("ssd").map(|v| v == "1").unwrap_or(false)
}

fn classify_disk_scsi_resolved(disk_conf: &ProxmoxDiskConf, resource: String) -> Arc<dyn ScsiDevice> {
    if is_cdrom(disk_conf) {
        Arc::new(Cdrom::new(resource))
    } else if is_ssd(disk_conf) {
        Arc::new(Ssd::new(resource))
    } else {
        Arc::new(Hdd::new(resource))
    }
}

fn classify_disk_sata_resolved(disk_conf: &ProxmoxDiskConf, resource: String) -> Arc<dyn SataDevice> {
    if is_cdrom(disk_conf) {
        Arc::new(Cdrom::new(resource))
    } else if is_ssd(disk_conf) {
        Arc::new(Ssd::new(resource))
    } else {
        Arc::new(Hdd::new(resource))
    }
}

fn classify_disk_ide_resolved(disk_conf: &ProxmoxDiskConf, resource: String) -> Arc<dyn IdeDevice> {
    if is_cdrom(disk_conf) {
        Arc::new(Cdrom::new(resource))
    } else if is_ssd(disk_conf) {
        Arc::new(Ssd::new(resource))
    } else {
        Arc::new(Hdd::new(resource))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use crate::config::proxmox::{ProxmoxVmConf, ProxmoxStorageConf};
    use crate::runtime::RootDeviceKind;

    fn make_importer(conf_str: &str) -> ProxmoxImporter {
        let vm_conf = ProxmoxVmConf::from_str(conf_str).unwrap();
        // Minimal storage conf with vm1-pool lvmthin so resolver succeeds
        let storage_str = "lvmthin: vm1-pool\n\tvgname vm1\n\tthinpool pool\n\tcontent images\n";
        let storage_conf = ProxmoxStorageConf::from_str(storage_str).unwrap();
        ProxmoxImporter::new(vm_conf, storage_conf, 108)
    }

    #[test]
    fn test_04_02_efidisk_logical_size_from_options() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\nefidisk0: vm1-pool:vm-108-efidisk,efitype=4m,pre-enrolled-keys=1,size=4M\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        let efidisk = runtime.root_devices().iter()
            .find(|d| d.device_kind() == RootDeviceKind::EfiDisk)
            .expect("EfiDisk not found");
        let efidisk = efidisk.as_any().downcast_ref::<EfiDisk>().unwrap();
        assert_eq!(efidisk.logical_size(), "4M");
        assert_eq!(efidisk.efitype().as_deref(), Some("4m"));
        assert!(efidisk.pre_enrolled_keys());
        assert!(efidisk.block_device_size_bytes().is_none());
    }

    #[test]
    fn test_04_02_tpmstate_version() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\ntpmstate0: vm1-pool:vm-108-tpmstate,size=4M\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        let tpm = runtime.root_devices().iter()
            .find(|d| d.device_kind() == RootDeviceKind::TpmState)
            .expect("TpmState not found");
        let tpm = tpm.as_any().downcast_ref::<TpmState>().unwrap();
        assert_eq!(tpm.version(), "v2.0");
    }

    #[test]
    fn test_04_02_hostpci_xvga_expands_to_two_functions() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\nhostpci0: 0000:03:00,pcie=1,x-vga=1\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        // HostPci lives inside Q35 chipset pcie bus — find it there
        let chipset = runtime.root_devices().iter()
            .find(|d| d.device_kind() == crate::runtime::RootDeviceKind::Chipset)
            .expect("Chipset not found");
        use crate::runtime::{Chipset, Q35Chipset};
        let chipset = chipset.as_any().downcast_ref::<Chipset>().unwrap();
        let q35 = match chipset {
            Chipset::Q35(q) => q,
            _ => panic!("expected Q35"),
        };
        let host_pci = q35.pcie_bus().values()
            .find_map(|d| d.as_any().downcast_ref::<HostPci>())
            .expect("HostPci not found in pcie_bus");
        assert_eq!(host_pci.base_bdf(), "0000:03:00");
        assert_eq!(*host_pci.functions(), vec![0u8, 1u8]);
    }

    #[test]
    fn test_07_01_boot_order_parsed_from_conf() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\nboot: order=scsi0;ide2;net0\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        assert_eq!(
            runtime.boot_order(),
            &vec!["scsi0".to_string(), "ide2".to_string(), "net0".to_string()]
        );
    }

    #[test]
    fn test_07_01_cpu_topology_parsed_from_conf() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\ncores: 8\nsockets: 1\ncpu: host\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        let cpu_device = runtime.root_devices().iter()
            .find(|d| d.device_kind() == RootDeviceKind::CpuTopology)
            .expect("CpuTopology not found");
        let cpu = cpu_device.as_any().downcast_ref::<crate::runtime::CpuTopology>().unwrap();
        assert_eq!(cpu.sockets(), &1);
        assert_eq!(cpu.cores(), &8);
        assert_eq!(cpu.cpu_type(), "host");
    }

    #[test]
    fn test_07_01_vga_none_parsed_from_conf_and_emits_flags() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\nvga: none\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        let vga_device = runtime.root_devices().iter()
            .find(|d| d.device_kind() == RootDeviceKind::VgaConfig)
            .expect("VgaConfig not found");
        let vga = vga_device.as_any().downcast_ref::<crate::runtime::VgaConfig>().unwrap();
        assert_eq!(vga.mode(), "none");

        use crate::config::qemu::{QemuCommandLine, QemuContext};
        let ctx = QemuContext::new(
            "testvm".to_string(),
            "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
            None,
        );
        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();
        let output = cmdline.to_string();
        assert!(output.contains("-vga none"), "output was: {output}");
        assert!(output.contains("-nographic"), "output was: {output}");
    }

    #[test]
    fn test_07_01_no_vga_key_produces_zero_vga_config_devices() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        let count = runtime.root_devices().iter()
            .filter(|d| d.device_kind() == RootDeviceKind::VgaConfig)
            .count();
        assert_eq!(count, 0);
    }

    fn q35_chipset(runtime: &crate::runtime::Runtime) -> crate::runtime::Q35Chipset {
        let chipset = runtime.root_devices().iter()
            .find(|d| d.device_kind() == RootDeviceKind::Chipset)
            .expect("Chipset not found");
        let chipset = chipset.as_any().downcast_ref::<Chipset>().unwrap();
        match chipset {
            Chipset::Q35(q) => q.clone(),
            _ => panic!("expected Q35"),
        }
    }

    #[test]
    fn test_07_03_net0_parsed_into_pcie_bus_virtio_net() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\nnet0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        let q35 = q35_chipset(&runtime);

        let nic = q35.pcie_bus().values()
            .find_map(|d| d.as_any().downcast_ref::<VirtioNetPcie>())
            .expect("VirtioNetPcie not found in pcie_bus");
        assert_eq!(nic.mac_address(), &Some("BC:24:11:3A:21:B7".to_string()));
    }

    #[test]
    fn test_07_03_usb0_parsed_into_usb_bus_host_passthrough() {
        let conf = "machine: pc-q35-8.1\nmemory: 4096\nusb0: host=1-2.2\n";
        let importer = make_importer(conf);
        let runtime = importer.into_runtime().unwrap();
        let q35 = q35_chipset(&runtime);

        let generic = q35.usb_bus().values()
            .find_map(|d| d.as_any().downcast_ref::<GenericUsbDevice>())
            .expect("GenericUsbDevice not found in usb_bus");
        assert_eq!(
            generic.kind(),
            &UsbDeviceKind::HostPassthrough { resource: "1-2.2".to_string() }
        );
    }

    #[test]
    fn test_07_03_felucia_108_net_and_usb_round_trip() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let conf_path = std::path::Path::new(&manifest).join("input/felucia/108.conf");
        let storage_path = std::path::Path::new(&manifest).join("input/felucia/storage.cfg");
        let conf_str = std::fs::read_to_string(conf_path).expect("108.conf not found");
        let storage_str = std::fs::read_to_string(storage_path).expect("storage.cfg not found");
        let vm_conf = ProxmoxVmConf::from_str(&conf_str).unwrap();
        let storage_conf = ProxmoxStorageConf::from_str(&storage_str).unwrap();
        let importer = ProxmoxImporter::new(vm_conf, storage_conf, 108);
        let runtime = importer.into_runtime().unwrap();
        let q35 = q35_chipset(&runtime);

        // pvscsi (existing scsi loop) + one virtio-net entry
        let net_count = q35.pcie_bus().values()
            .filter(|d| d.as_any().downcast_ref::<VirtioNetPcie>().is_some())
            .count();
        assert_eq!(net_count, 1, "expected exactly one VirtioNetPcie in pcie_bus");
        assert_eq!(q35.usb_bus().len(), 1, "expected exactly one usb_bus entry");
    }
}
