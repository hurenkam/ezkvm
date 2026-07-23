use ezkvm::runtime::{
    AudioDevice, EfiDisk, HostPci, Ivshmem, Q35ChipsetBuilder, RawArgs,
    RuntimeBuilder, SpiceDisplay, TpmState,
};
use ezkvm::runtime::PcieBusDeviceKind;
use std::sync::Arc;

#[test]
fn test_felucia_108_runtime_constructs() {
    let efidisk = EfiDisk::new(
        "vm1-pool:vm-108-efidisk".to_string(),
        Some("4m".to_string()),
        true,
        Some("2023".to_string()),
        "4M".to_string(),
        Some(540672),
    );
    let tpmstate = TpmState::new("vm1-pool:vm-108-tpmstate".to_string(), "v2.0".to_string());
    let audio = AudioDevice::new("ich9-intel-hda".to_string(), "spice".to_string());
    let spice = SpiceDisplay::new(Some(5903), Some("0.0.0.0".to_string()), true, false, None, false);
    let raw_args = RawArgs(
        "-device virtio-serial-pci -chardev spicevmc,id=vdagent,name=vdagent".to_string(),
    );

    let runtime = RuntimeBuilder::new()
        .with_efidisk(efidisk)
        .with_tpmstate(tpmstate)
        .with_audio_device(audio)
        .with_spice_display(spice)
        .with_raw_args(raw_args)
        .build()
        .unwrap();

    assert_eq!(runtime.root_devices().len(), 5);

    let host_pci = HostPci::new("0000:03:00".to_string(), vec![0, 1], true, true, None, None);
    let ivshmem = Ivshmem::new("ivshmem0".to_string(), "/dev/kvmfr0".to_string(), "128M".to_string());

    let chipset = Q35ChipsetBuilder::new()
        .with_host_pci(0, Arc::new(host_pci))
        .with_ivshmem(0, Arc::new(ivshmem))
        .build();

    assert_eq!(chipset.pcie_bus().len(), 2);
}

#[test]
fn test_efidisk_dual_size_fields_are_independent() {
    let efidisk = EfiDisk::new(
        "pool:vm-108-efidisk".to_string(),
        Some("4m".to_string()),
        true,
        None,
        "4M".to_string(),
        Some(540672),
    );
    assert_eq!(efidisk.logical_size(), "4M");
    assert_eq!(*efidisk.block_device_size_bytes(), Some(540672u64));
    assert_ne!(540672u64, 4 * 1024 * 1024);
}

#[test]
fn test_hostpci_multi_function_stored_as_vec() {
    let host_pci = HostPci::new("0000:03:00".to_string(), vec![0, 1], true, true, None, None);
    assert_eq!(host_pci.functions().len(), 2);
}

#[test]
fn test_rawargs_string_is_verbatim() {
    let raw = RawArgs("hello -device foo,bar".to_string());
    assert_eq!(raw.0, "hello -device foo,bar");
}

#[test]
fn test_ivshmem_is_pcie_device_kind() {
    use ezkvm::runtime::PcieDevice;
    let ivshmem = Ivshmem::new("ivshmem0".to_string(), "/dev/kvmfr0".to_string(), "128M".to_string());
    assert_eq!(ivshmem.device_kind(), PcieBusDeviceKind::Ivshmem);
}
