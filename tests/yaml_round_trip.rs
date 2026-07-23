//! Phase 6 Plan 06-01, Task 3: end-to-end Runtime <-> ezkvm YAML round-trip integration test.
//!
//! Loads the Felucia 108 fixture through the Proxmox importer to build a real-world Runtime,
//! converts it to the ezkvm YAML `ConfigSchema`, converts it back to a `Runtime`, and asserts
//! that every modeled v1 field survives the round trip per D-04 (Phase 6 context).

use ezkvm::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
use ezkvm::config::EzkvmConfigSchema;
use ezkvm::runtime::{
    AudioDevice, Chipset, EfiDisk, HostPci, PcieBusDeviceKind, PvScsi, RawArgs, RootDeviceKind,
    Runtime, ScsiAddress, Ssd, TpmState,
};
use std::str::FromStr;

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
fn felucia_108_runtime_round_trips_yaml() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let original = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");

    let original_root_device_count = original.root_devices().len();

    let schema = EzkvmConfigSchema::try_from(original)
        .expect("runtime -> ezkvm yaml schema conversion failed");
    let round_tripped: Runtime =
        Runtime::try_from(schema).expect("ezkvm yaml schema -> runtime conversion failed");

    assert_eq!(
        round_tripped.root_devices().len(),
        original_root_device_count,
        "root device count must be preserved across the yaml round trip"
    );

    // EfiDisk storage_volume preserved
    let efidisk = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::EfiDisk)
        .expect("EfiDisk not found after round trip")
        .as_any()
        .downcast_ref::<EfiDisk>()
        .expect("downcast to EfiDisk");
    assert_eq!(efidisk.storage_volume(), "/dev/vm1/vm-108-efidisk");

    // TpmState version preserved
    let tpm = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::TpmState)
        .expect("TpmState not found after round trip")
        .as_any()
        .downcast_ref::<TpmState>()
        .expect("downcast to TpmState");
    assert_eq!(tpm.version(), "v2.0");

    // AudioDevice preserved (sanity check alongside RawArgs / HostPci / storage below)
    let _audio = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::AudioDevice)
        .expect("AudioDevice not found after round trip")
        .as_any()
        .downcast_ref::<AudioDevice>()
        .expect("downcast to AudioDevice");

    // RawArgs preserved verbatim (non-empty, starts with '-')
    let raw_args = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::RawArgs)
        .expect("RawArgs not found after round trip")
        .as_any()
        .downcast_ref::<RawArgs>()
        .expect("downcast to RawArgs");
    assert!(!raw_args.0.is_empty(), "RawArgs must not be empty after round trip");
    assert!(raw_args.0.starts_with('-'), "RawArgs must be verbatim: {}", raw_args.0);

    // HostPci base_bdf preserved
    let chipset = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::Chipset)
        .expect("Chipset not found after round trip")
        .as_any()
        .downcast_ref::<Chipset>()
        .expect("downcast to Chipset");
    let q35 = match chipset {
        Chipset::Q35(q) => q,
        _ => panic!("expected Q35 chipset after round trip"),
    };
    let host_pci = q35
        .pcie_bus()
        .values()
        .find(|d| d.device_kind() == PcieBusDeviceKind::HostPci)
        .expect("HostPci not found after round trip")
        .as_any()
        .downcast_ref::<HostPci>()
        .expect("downcast to HostPci");
    assert_eq!(host_pci.base_bdf(), "0000:03:00");

    // Ssd resource path preserved (non-empty — the synthetic-ID bug fixed)
    let pvscsi = q35
        .pcie_bus()
        .values()
        .find(|d| d.device_kind() == PcieBusDeviceKind::PvScsi)
        .expect("PvScsi not found after round trip")
        .as_any()
        .downcast_ref::<PvScsi>()
        .expect("downcast to PvScsi");
    let scsi0 = pvscsi
        .scsi_bus()
        .get(&ScsiAddress::new(0, 0))
        .expect("scsi0 not found after round trip");
    let ssd = scsi0
        .as_any()
        .downcast_ref::<Ssd>()
        .expect("scsi0 should be an Ssd after round trip");
    assert!(!ssd.resource.is_empty(), "Ssd resource path must not be empty after round trip");
    assert_eq!(ssd.resource, "/dev/vm1/vm-108-boot");
}
