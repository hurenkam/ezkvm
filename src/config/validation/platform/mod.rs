mod audio;
mod core;
mod devices;
mod helpers;
mod usb;

pub(crate) use audio::validate_audio_devices;
pub(crate) use core::{
    validate_ballooning_config, validate_guest_agent_config, validate_hostpci_config,
    validate_hyperv_config, validate_qmp_config, validate_smbios_config, validate_spice_config,
    validate_tpm_config,
};
pub(crate) use devices::{
    validate_hugepages_config, validate_input_devices, validate_iommu_config,
    validate_iscsi_disk_config, validate_ivshmem_config, validate_numa_config,
    validate_sata_controller_config, validate_scsi_controller_config,
};
pub(crate) use usb::{validate_usb_device_config, validate_xhci_controller_config};
