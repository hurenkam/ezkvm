use serde::{Deserialize, Serialize};

use super::super::{
    AudioDeviceConfig, BallooningConfig, GuestAgentConfig, HostPciConfig, HypervConfig,
    InputDeviceConfig, IscsiDiskConfig, IvshmemConfig, NumaConfig, QmpConfig, ScsiControllerConfig,
    SmbiosConfig, SpiceConfig, TpmConfig, UsbDeviceConfig, VmOptions, XhciControllerConfig,
};
use super::{BootConfig, DeviceConfig, SystemConfig};

/// Main configuration structure for a virtual machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Name of the virtual machine
    pub name: String,

    /// Backend to use (currently only "qemu" is supported)
    pub backend: String,

    /// Optional list of profile names to layer before applying VM overrides.
    /// Profile files are resolved from the configured profile directory.
    #[serde(default)]
    pub profiles: Vec<String>,

    /// System configuration (CPU, memory, etc.)
    pub system: SystemConfig,

    /// Boot configuration
    #[serde(default)]
    pub boot: BootConfig,

    /// Device configuration
    #[serde(default)]
    pub devices: DeviceConfig,

    /// TPM configuration
    #[serde(default)]
    pub tpm: Option<TpmConfig>,

    /// Guest agent configuration
    #[serde(default)]
    pub guest_agent: Option<GuestAgentConfig>,

    /// Memory ballooning configuration
    #[serde(default)]
    pub ballooning: Option<BallooningConfig>,

    /// Hardware passthrough configuration
    #[serde(default)]
    pub hostpci: Vec<HostPciConfig>,

    /// USB device passthrough configuration
    #[serde(default)]
    pub usb_devices: Vec<UsbDeviceConfig>,

    /// XHCI controller configuration
    #[serde(default)]
    pub xhci_controllers: Vec<XhciControllerConfig>,

    /// SPICE display configuration
    #[serde(default)]
    pub spice: Option<SpiceConfig>,

    /// Audio devices backed by the selected audio backend
    #[serde(default)]
    pub audio_devices: Vec<AudioDeviceConfig>,

    /// Explicit input devices
    #[serde(default)]
    pub input_devices: Vec<InputDeviceConfig>,

    /// Looking Glass shared memory configuration
    #[serde(default)]
    pub ivshmem: Option<IvshmemConfig>,

    /// SCSI controller configuration
    #[serde(default)]
    pub scsi_controllers: Vec<ScsiControllerConfig>,

    /// iSCSI storage configuration
    #[serde(default)]
    pub iscsi_disks: Vec<IscsiDiskConfig>,

    /// QMP monitoring configuration
    #[serde(default)]
    pub qmp: Option<QmpConfig>,

    /// SMBIOS system information configuration
    #[serde(default)]
    pub smbios: Option<SmbiosConfig>,

    /// NUMA topology configuration
    #[serde(default)]
    pub numa: Vec<NumaConfig>,

    /// Hyper-V enlightenments configuration
    #[serde(default)]
    pub hyperv: Option<HypervConfig>,

    /// Additional options
    #[serde(default)]
    pub options: VmOptions,
}
