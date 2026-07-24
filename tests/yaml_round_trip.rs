//! Phase 6 Plan 06-01, Task 3: end-to-end Runtime <-> ezkvm YAML round-trip integration test.
//!
//! Loads the Felucia 108 fixture through the Proxmox importer to build a real-world Runtime,
//! converts it to the ezkvm YAML `ConfigSchema`, serializes and parses YAML, converts it back
//! to a `Runtime`, and asserts that every modeled v1 field survives the round trip per D-04
//! (Phase 6 context).

use ezkvm::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
use ezkvm::config::EzkvmConfigSchema;
use ezkvm::runtime::{
    AudioDevice, Chipset, EfiDisk, HostPci, Memory, PcieBusDeviceKind, PvScsi, RawArgs,
    RootDeviceKind, Runtime, RuntimeBuilder, ScsiAddress, SpiceDisplay, Ssd, TpmState,
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
    let original_root_device_kinds = original
        .root_devices()
        .iter()
        .map(|device| device.device_kind())
        .collect::<Vec<_>>();
    let original_efidisk = original
        .root_devices()
        .iter()
        .find(|device| device.device_kind() == RootDeviceKind::EfiDisk)
        .expect("original EfiDisk not found")
        .as_any()
        .downcast_ref::<EfiDisk>()
        .expect("downcast original EfiDisk");
    let expected_efidisk = (
        original_efidisk.storage_volume().clone(),
        original_efidisk.efitype().clone(),
        *original_efidisk.pre_enrolled_keys(),
        original_efidisk.ms_cert().clone(),
        original_efidisk.logical_size().clone(),
    );
    let original_tpm = original
        .root_devices()
        .iter()
        .find(|device| device.device_kind() == RootDeviceKind::TpmState)
        .expect("original TpmState not found")
        .as_any()
        .downcast_ref::<TpmState>()
        .expect("downcast original TpmState");
    let expected_tpm = (
        original_tpm.storage_volume().clone(),
        original_tpm.version().clone(),
    );
    let original_audio = original
        .root_devices()
        .iter()
        .find(|device| device.device_kind() == RootDeviceKind::AudioDevice)
        .expect("original AudioDevice not found")
        .as_any()
        .downcast_ref::<AudioDevice>()
        .expect("downcast original AudioDevice");
    let expected_audio = (
        original_audio.device_type().clone(),
        original_audio.driver().clone(),
    );
    let expected_raw_args = original
        .root_devices()
        .iter()
        .find(|device| device.device_kind() == RootDeviceKind::RawArgs)
        .expect("original RawArgs not found")
        .as_any()
        .downcast_ref::<RawArgs>()
        .expect("downcast original RawArgs")
        .0
        .clone();
    let original_chipset = original
        .root_devices()
        .iter()
        .find(|device| device.device_kind() == RootDeviceKind::Chipset)
        .expect("original Chipset not found")
        .as_any()
        .downcast_ref::<Chipset>()
        .expect("downcast original Chipset");
    let original_q35 = match original_chipset {
        Chipset::Q35(q35) => q35,
        _ => panic!("expected original Q35 chipset"),
    };
    let expected_pcie_topology = original_q35
        .pcie_bus()
        .iter()
        .map(|(address, device)| {
            (
                (*address.device(), *address.function()),
                device.device_kind(),
            )
        })
        .collect::<Vec<_>>();
    let original_host_pci = original_q35
        .pcie_bus()
        .values()
        .find(|device| device.device_kind() == PcieBusDeviceKind::HostPci)
        .expect("original HostPci not found")
        .as_any()
        .downcast_ref::<HostPci>()
        .expect("downcast original HostPci");
    let expected_host_pci = (
        original_host_pci.base_bdf().clone(),
        original_host_pci.functions().clone(),
        *original_host_pci.x_vga(),
        original_host_pci.rombar().clone(),
        original_host_pci.romfile().clone(),
    );
    let original_pvscsi = original_q35
        .pcie_bus()
        .values()
        .find(|device| device.device_kind() == PcieBusDeviceKind::PvScsi)
        .expect("original PvScsi not found")
        .as_any()
        .downcast_ref::<PvScsi>()
        .expect("downcast original PvScsi");
    let expected_scsi0_resource = original_pvscsi
        .scsi_bus()
        .get(&ScsiAddress::new(0, 0))
        .expect("original scsi0 not found")
        .as_any()
        .downcast_ref::<Ssd>()
        .expect("original scsi0 should be an Ssd")
        .resource
        .clone();

    let schema = EzkvmConfigSchema::try_from(original)
        .expect("runtime -> ezkvm yaml schema conversion failed");
    let yaml = schema
        .to_styled_compact_yaml()
        .expect("ezkvm yaml serialization failed");
    assert!(!yaml.is_empty(), "serialized ezkvm YAML must not be empty");
    let parsed_schema =
        EzkvmConfigSchema::from_str(&yaml).expect("ezkvm yaml schema parsing failed");
    let round_tripped: Runtime = Runtime::try_from(parsed_schema)
        .expect("ezkvm yaml schema -> runtime conversion failed");

    assert_eq!(
        round_tripped.root_devices().len(),
        original_root_device_count,
        "root device count must be preserved across the yaml round trip"
    );
    assert_eq!(
        round_tripped
            .root_devices()
            .iter()
            .map(|device| device.device_kind())
            .collect::<Vec<_>>(),
        original_root_device_kinds,
        "root device types and ordering must be preserved across the yaml round trip"
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
    assert_eq!(efidisk.storage_volume(), &expected_efidisk.0);
    assert_eq!(efidisk.efitype(), &expected_efidisk.1);
    assert_eq!(efidisk.pre_enrolled_keys(), &expected_efidisk.2);
    assert_eq!(efidisk.ms_cert(), &expected_efidisk.3);
    assert_eq!(efidisk.logical_size(), &expected_efidisk.4);

    // TpmState version preserved
    let tpm = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::TpmState)
        .expect("TpmState not found after round trip")
        .as_any()
        .downcast_ref::<TpmState>()
        .expect("downcast to TpmState");
    assert_eq!(tpm.storage_volume(), &expected_tpm.0);
    assert_eq!(tpm.version(), &expected_tpm.1);

    let audio = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::AudioDevice)
        .expect("AudioDevice not found after round trip")
        .as_any()
        .downcast_ref::<AudioDevice>()
        .expect("downcast to AudioDevice");
    assert_eq!(audio.device_type(), &expected_audio.0);
    assert_eq!(audio.driver(), &expected_audio.1);

    let raw_args = round_tripped
        .root_devices()
        .iter()
        .find(|d| d.device_kind() == RootDeviceKind::RawArgs)
        .expect("RawArgs not found after round trip")
        .as_any()
        .downcast_ref::<RawArgs>()
        .expect("downcast to RawArgs");
    assert_eq!(raw_args.0, expected_raw_args, "RawArgs must be verbatim");

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
    let mut round_tripped_pcie_topology = q35
        .pcie_bus()
        .iter()
        .map(|(address, device)| {
            (
                (*address.device(), *address.function()),
                device.device_kind(),
            )
        })
        .collect::<Vec<_>>();
    round_tripped_pcie_topology.sort_unstable_by_key(|(address, _)| *address);
    let mut expected_pcie_topology = expected_pcie_topology;
    expected_pcie_topology.sort_unstable_by_key(|(address, _)| *address);
    assert_eq!(
        round_tripped_pcie_topology,
        expected_pcie_topology,
        "PCIe device addresses and types must be preserved across the yaml round trip"
    );
    let host_pci = q35
        .pcie_bus()
        .values()
        .find(|d| d.device_kind() == PcieBusDeviceKind::HostPci)
        .expect("HostPci not found after round trip")
        .as_any()
        .downcast_ref::<HostPci>()
        .expect("downcast to HostPci");
    assert_eq!(host_pci.base_bdf(), &expected_host_pci.0);
    assert_eq!(host_pci.functions(), &expected_host_pci.1);
    assert_eq!(host_pci.x_vga(), &expected_host_pci.2);
    assert_eq!(host_pci.rombar(), &expected_host_pci.3);
    assert_eq!(host_pci.romfile(), &expected_host_pci.4);

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
    assert_eq!(ssd.resource, expected_scsi0_resource);
}

#[test]
fn spice_display_round_trips_yaml() {
    let expected = SpiceDisplay::new(
        Some(5903),
        Some("0.0.0.0".to_string()),
        true,
        true,
        Some("/dev/dri/renderD128".to_string()),
        false,
    );
    let runtime = RuntimeBuilder::new()
        .with_memory(Memory::new(4096))
        .with_chipset(Chipset::I440FX)
        .with_spice_display(expected.clone())
        .build()
        .expect("build SPICE runtime");
    let schema = EzkvmConfigSchema::try_from(runtime).expect("runtime -> schema");
    let yaml = schema
        .to_styled_compact_yaml()
        .expect("serialize SPICE YAML");
    let parsed_schema = EzkvmConfigSchema::from_str(&yaml).expect("parse SPICE YAML");
    let round_tripped = Runtime::try_from(parsed_schema).expect("schema -> runtime");
    let spice = round_tripped
        .root_devices()
        .iter()
        .find(|device| device.device_kind() == RootDeviceKind::SpiceDisplay)
        .expect("SPICE display not found after YAML round trip")
        .as_any()
        .downcast_ref::<SpiceDisplay>()
        .expect("downcast SPICE display");

    assert_eq!(spice.port(), expected.port());
    assert_eq!(spice.addr(), expected.addr());
    assert_eq!(spice.disable_ticketing(), expected.disable_ticketing());
    assert_eq!(spice.gl(), expected.gl());
    assert_eq!(spice.rendernode(), expected.rendernode());
    assert_eq!(spice.clipboard(), expected.clipboard());
}

#[test]
fn empty_resources_round_trip_yaml() {
    let runtime = RuntimeBuilder::new()
        .with_memory(Memory::new(4096))
        .with_chipset(Chipset::I440FX)
        .build()
        .expect("build empty-resource runtime");
    let schema = EzkvmConfigSchema::try_from(runtime).expect("runtime -> schema");
    let yaml = schema
        .to_styled_compact_yaml()
        .expect("serialize empty-resource YAML");
    assert!(yaml.contains("resources: []"));
    assert!(yaml.contains("devices: []"));
    let parsed_schema =
        EzkvmConfigSchema::from_str(&yaml).expect("parse empty-resource YAML");
    Runtime::try_from(parsed_schema).expect("schema -> runtime");
}
