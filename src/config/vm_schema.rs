use serde::{Deserialize, Serialize};

use super::{
    AudioDeviceConfig, BallooningConfig, GuestAgentConfig, HostPciConfig, HypervConfig,
    InputDeviceConfig, IscsiDiskConfig, IvshmemConfig, NumaConfig, QmpConfig, ScsiControllerConfig,
    SmbiosConfig, SpiceConfig, TpmConfig, UsbDeviceConfig, VmOptions, XhciControllerConfig,
};

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

/// System-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Target architecture (x86_64, aarch64, etc.)
    pub architecture: String,

    /// Machine type (q35, pc, virt, etc.)
    pub machine: String,

    /// Additional machine-specific options appended to `-machine`
    #[serde(default)]
    pub machine_options: Vec<String>,

    /// Memory in MiB
    pub memory: u32,

    /// Number of virtual CPUs
    pub vcpus: u32,

    /// CPU model to emulate
    pub cpu_model: String,

    /// CPU-specific features
    #[serde(default)]
    pub cpu_features: Vec<CpuFeature>,

    /// Optional QEMU config file(s) to load via -readconfig.
    /// Use this to supply machine topology files such as
    /// /usr/share/qemu-server/pve-q35-4.0.cfg which define the
    /// PCI/PCIe bridge buses (pci.0, pci.1, ich9-pcie-port-*, etc.)
    #[serde(default)]
    pub readconfig: Vec<String>,
}

/// CPU feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuFeature {
    /// Feature name (e.g., "+vmx", "-avx")
    pub name: String,
}

/// Boot configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootConfig {
    /// Firmware type (uefi, bios, or ovmf)
    #[serde(default)]
    pub firmware: Option<String>,

    /// Boot order (disk, cdrom, network)
    #[serde(default)]
    pub boot_order: Vec<String>,

    /// Show the boot menu
    #[serde(default)]
    pub menu: bool,

    /// Enforce strict boot ordering
    #[serde(default)]
    pub strict: bool,

    /// Reboot timeout in milliseconds
    pub reboot_timeout: Option<u32>,

    /// Splash screen image path
    pub splash: Option<String>,

    /// Kernel path (optional)
    pub kernel: Option<String>,

    /// Initrd path (optional)
    pub initrd: Option<String>,

    /// Kernel command line
    pub cmdline: Option<String>,

    /// UEFI firmware code path (for custom OVMF)
    pub uefi_code: Option<String>,

    /// UEFI variables path
    pub uefi_vars: Option<String>,

    /// Optional explicit UEFI variables drive size in bytes for pflash unit 1.
    /// Proxmox uses `size=540672` for OVMF vars even when the backing device is larger.
    pub uefi_vars_size: Option<u64>,

    /// Enable secure boot
    #[serde(default)]
    pub secure_boot: bool,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    /// Storage devices
    #[serde(default)]
    pub drives: Vec<DriveConfig>,

    /// Network devices
    #[serde(default)]
    pub networks: Vec<NetworkConfig>,

    /// Display devices
    #[serde(default)]
    pub displays: Vec<DisplayConfig>,

    /// Serial devices
    #[serde(default)]
    pub serials: Vec<SerialConfig>,
}

/// Drive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveConfig {
    /// Unique identifier for the drive
    pub id: String,

    /// Path to the disk image
    pub path: String,

    /// Interface type (virtio, scsi, ide, nvme)
    pub interface: String,

    /// Drive type (disk, cdrom)
    pub r#type: String,

    /// Image format (qcow2, raw, etc.)
    pub format: String,

    /// Whether the drive is read-only
    #[serde(default)]
    pub readonly: bool,

    /// Enable discard (TRIM) support
    #[serde(default)]
    pub discard: bool,

    /// Enable SSD emulation
    #[serde(default)]
    pub ssd: bool,

    /// QEMU cache mode
    pub cache: Option<String>,

    /// QEMU async I/O backend
    pub aio: Option<String>,

    /// Detect-zeroes behavior
    pub detect_zeroes: Option<String>,

    /// SCSI controller to attach to (for SCSI drives)
    pub controller: Option<String>,

    /// Boot index for firmware boot ordering
    pub boot_index: Option<u32>,

    /// SCSI target ID for attached SCSI devices
    pub scsi_id: Option<u32>,

    /// Explicit attachment bus for device-based drive emission
    pub bus: Option<String>,

    /// Unit number for IDE/SATA style drive placement
    pub unit: Option<u32>,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Unique identifier for the network device
    pub id: String,

    /// Network model (virtio-net, e1000, etc.)
    pub model: String,

    /// Network mode (user, bridge, socket)
    pub mode: String,

    /// MAC address
    pub mac: Option<String>,

    /// RX queue size
    pub rx_queue_size: Option<u32>,

    /// TX queue size
    pub tx_queue_size: Option<u32>,

    /// Boot index for firmware boot ordering
    pub boot_index: Option<u32>,

    /// PCI/PCIe bus placement for the network device
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    pub addr: Option<String>,
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Display type (virtio-gpu, qxl, cirrus)
    pub r#type: String,

    /// Video RAM in MiB
    #[serde(default)]
    pub vram: Option<u32>,
}

/// Serial configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    /// Serial type (pty, file, socket, stdio)
    pub r#type: String,

    /// Port number (for multi-port setups)
    #[serde(default)]
    pub port: Option<u32>,

    /// Output file path for `file` serial backends
    pub path: Option<String>,

    /// Hostname or IP for `socket` serial backends
    pub host: Option<String>,

    /// TCP port for `socket` serial backends
    pub socket_port: Option<u16>,

    /// Whether the socket backend should listen in server mode
    #[serde(default = "default_true")]
    pub server: bool,

    /// Whether QEMU should wait for a socket connection before continuing
    #[serde(default)]
    pub wait: bool,
}

fn default_true() -> bool {
    true
}
