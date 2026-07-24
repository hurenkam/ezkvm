use ezkvm::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
use ezkvm::runtime::{
    AudioDevice, Chipset, EfiDisk, HostPci, PcieAddress, RawArgs, Ssd, TpmState,
    RootDeviceKind,
};
use std::str::FromStr;
use std::sync::Arc;

fn load_felucia_108() -> (ProxmoxVmConf, ProxmoxStorageConf) {
    let conf_str = std::fs::read_to_string("input/felucia/108.conf")
        .expect("input/felucia/108.conf not found");
    let storage_str = std::fs::read_to_string("input/felucia/storage.cfg")
        .expect("input/felucia/storage.cfg not found");
    let vm_conf = ProxmoxVmConf::from_str(&conf_str).expect("parse 108.conf");
    let storage_conf = ProxmoxStorageConf::from_str(&storage_str).expect("parse storage.cfg");
    (vm_conf, storage_conf)
}

#[test]
fn test_proxmox_import_felucia_108_root_devices() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    // Memory + Chipset + EfiDisk + TpmState + AudioDevice + RawArgs + CpuTopology + VgaConfig
    // (CpuTopology/VgaConfig added Phase 7 Plan 07-01 for -smp/-cpu/-vga emission, D-02/D-07)
    assert_eq!(runtime.root_devices().len(), 8);
}

#[test]
fn test_proxmox_import_felucia_108_efidisk() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    let efidisk = runtime
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::EfiDisk)
        .expect("EfiDisk not found")
        .as_any()
        .downcast_ref::<EfiDisk>()
        .expect("downcast to EfiDisk");
    assert_eq!(efidisk.storage_volume(), "/dev/vm1/vm-108-efidisk");
    assert_eq!(efidisk.logical_size(), "4M");
}

#[test]
fn test_proxmox_import_felucia_108_tpmstate() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    let tpm = runtime
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::TpmState)
        .expect("TpmState not found")
        .as_any()
        .downcast_ref::<TpmState>()
        .expect("downcast to TpmState");
    assert_eq!(tpm.version(), "v2.0");
}

#[test]
fn test_proxmox_import_felucia_108_audio() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    let audio = runtime
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::AudioDevice)
        .expect("AudioDevice not found")
        .as_any()
        .downcast_ref::<AudioDevice>()
        .expect("downcast to AudioDevice");
    assert_eq!(audio.device_type(), "ich9-intel-hda");
    assert_eq!(audio.driver(), "spice");
}

#[test]
fn test_proxmox_import_felucia_108_raw_args() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    let raw = runtime
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::RawArgs)
        .expect("RawArgs not found")
        .as_any()
        .downcast_ref::<RawArgs>()
        .expect("downcast to RawArgs");
    assert!(raw.0.contains("-spice port=5903"), "expected spice args in RawArgs");
}

#[test]
fn test_proxmox_import_felucia_108_hostpci() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    let chipset = runtime
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::Chipset)
        .expect("Chipset not found")
        .as_any()
        .downcast_ref::<Chipset>()
        .expect("downcast to Chipset");
    let q35 = match chipset {
        Chipset::Q35(q) => q,
        _ => panic!("expected Q35 chipset"),
    };
    let host_pci = q35
        .pcie_bus()
        .get(&PcieAddress::new(0, 0))
        .expect("hostpci0 not at PcieAddress(0,0)")
        .as_any()
        .downcast_ref::<HostPci>()
        .expect("downcast to HostPci");
    assert_eq!(host_pci.base_bdf(), "0000:03:00");
    assert_eq!(host_pci.functions().len(), 2);
}

#[test]
fn test_proxmox_import_felucia_108_scsi0_resource() {
    use ezkvm::runtime::{PvScsi, ScsiAddress};
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    let chipset = runtime
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::Chipset)
        .expect("Chipset not found")
        .as_any()
        .downcast_ref::<Chipset>()
        .expect("downcast to Chipset");
    let q35 = match chipset {
        Chipset::Q35(q) => q,
        _ => panic!("expected Q35 chipset"),
    };
    let pvscsi = q35
        .pcie_bus()
        .get(&PcieAddress::new(16, 0))
        .expect("PvScsi not at PcieAddress(16,0)")
        .as_any()
        .downcast_ref::<PvScsi>()
        .expect("downcast to PvScsi");
    let scsi0 = pvscsi
        .scsi_bus()
        .get(&ScsiAddress::new(0, 0))
        .expect("scsi0 not at ScsiAddress(0,0)");
    let ssd = scsi0
        .as_any()
        .downcast_ref::<Ssd>()
        .expect("scsi0 should be an Ssd");
    assert_eq!(ssd.resource, "/dev/vm1/vm-108-boot");
}
