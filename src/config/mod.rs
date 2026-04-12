//! Configuration module for ezkvm
//!
//! This module handles parsing and validation of YAML configuration files
//! that define virtual machine specifications.

pub mod system;
pub mod devices;
pub mod validation;

use serde::{Deserialize, Serialize};
use std::path::Path;

const DEFAULT_CENTRAL_CONFIG_PATHS: &[&str] = &[
    "/etc/ezkvm/ezkvm.yaml",
    "/etc/ezkvm.yaml",
];

const DEFAULT_PROFILE_DIR: &str = "/etc/ezkvm/profiles.d";

/// Central tool configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CentralConfig {
    /// Tool paths
    #[serde(default)]
    pub tools: ToolsConfig,
    
    /// Directory locations
    #[serde(default)]
    pub locations: LocationsConfig,

    /// Looking Glass client options
    #[serde(default)]
    pub looking_glass: LookingGlassOptions,
}

/// Tool paths configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    /// Path to swtpm executable
    pub swtpm: Option<String>,
    
    /// Path to remote-viewer executable
    pub remote_viewer: Option<String>,
    
    /// Path to looking-glass-client executable
    pub looking_glass: Option<String>,
}

/// Directory locations configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocationsConfig {
    /// Runtime directory for PID files, sockets, etc.
    pub run_dir: Option<String>,
    
    /// Directory containing OVMF firmware files
    pub ovmf_dir: Option<String>,
    
    /// Default directory for VM configuration files
    pub vm_dir: Option<String>,
    
    /// Directory for VM templates
    pub template_dir: Option<String>,

    /// Directory containing reusable VM profile files
    pub profile_dir: Option<String>,
}

/// Looking Glass client options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LookingGlassOptions {
    /// Launch the client in fullscreen mode.
    pub full_screen: Option<bool>,

    /// Initial window size in WIDTHxHEIGHT format.
    pub size: Option<String>,

    /// Grab the keyboard when focused.
    pub grab_keyboard: Option<bool>,

    /// Escape key name used to release keyboard grab.
    pub escape_key: Option<String>,
}

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

/// Additional VM options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmOptions {
    /// Enable KVM acceleration
    pub enable_kvm: bool,
    
    /// Run in daemon mode
    pub daemonize: bool,

    /// Disable QEMU default devices
    #[serde(default)]
    pub nodefaults: bool,

    /// Raw `-global` options
    #[serde(default)]
    pub global_options: Vec<String>,

    /// RTC configuration
    #[serde(default)]
    pub rtc: Option<RtcConfig>,

    /// Custom PID file location
    pub pid_file: Option<String>,

    /// Custom log directory for VM-specific logs
    pub log_dir: Option<String>,

    /// Number of log files to retain during rotation
    pub log_keep: Option<usize>,
    
    /// Path to UEFI variables file
    pub uefi_vars: Option<String>,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            enable_kvm: true,
            daemonize: false,
            nodefaults: false,
            global_options: Vec::new(),
            rtc: None,
            pid_file: None,
            log_dir: None,
            log_keep: None,
            uefi_vars: None,
        }
    }
}

/// RTC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcConfig {
    /// RTC base, usually `utc` or `localtime`
    pub base: Option<String>,

    /// RTC drift fix policy, usually `slew` or `none`
    pub driftfix: Option<String>,
}

/// TPM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmConfig {
    /// TPM version (1.2 or 2.0)
    pub version: String,
    
    /// TPM backend type (emulator or passthrough)
    pub backend: String,
    
    /// Path to TPM socket file (overrides the default run-dir path)
    pub state_path: Option<String>,

    /// Directory containing swtpm state files.
    /// When set, swtpm is invoked with --tpmstate dir=<state_dir> using this path
    /// instead of the default empty run-dir subdirectory.
    /// Use this to point at a mounted Proxmox TPM-state disk so Windows keeps its
    /// existing BitLocker keys (e.g. mount /dev/vm1/vm-108-tpmstate → /mnt/tpmstate).
    pub state_dir: Option<String>,

    /// URI for swtpm state backend, passed as `--tpmstate backend-uri=<uri>`.
    /// This can be used instead of mounting a TPM-state volume and configuring
    /// `state_dir`.
    /// Example: `file:///dev/vm1/vm-108-tpmstate`
    #[serde(alias = "state_backend_url")]
    pub state_backend_uri: Option<String>,
    
    /// Device model (tpm-tis or tpm-crb)
    #[serde(default = "default_tpm_model")]
    pub model: String,
}

fn default_tpm_model() -> String {
    "tpm-tis".to_string()
}

/// Guest agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestAgentConfig {
    /// Enable guest agent
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Path to guest agent socket
    pub socket_path: Option<String>,
    
    /// Freeze CPU on suspend
    #[serde(default)]
    pub freeze_cpu: bool,

    /// PCI/PCIe bus placement for the virtio-serial controller
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    pub addr: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Memory ballooning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallooningConfig {
    /// Enable memory ballooning
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Enable free page reporting
    #[serde(default)]
    pub free_page_reporting: bool,
    
    /// Balloon device model
    #[serde(default = "default_balloon_model")]
    pub model: String,

    /// Optional device identifier
    pub id: Option<String>,

    /// PCI/PCIe bus placement for the balloon device
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    pub addr: Option<String>,
}

fn default_balloon_model() -> String {
    "virtio-balloon-pci".to_string()
}

/// Hardware passthrough configuration for PCI devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostPciConfig {
    /// PCI device address (e.g., "0000:03:00.0")
    pub device: String,
    
    /// Unique identifier for the device
    pub id: String,
    
    /// PCIe configuration
    #[serde(default)]
    pub pcie: bool,
    
    /// VGA passthrough (for GPU devices)
    #[serde(default)]
    pub x_vga: bool,

    /// Optional guest bus placement for the passthrough device
    pub bus: Option<String>,

    /// Optional guest slot/function address for the passthrough device
    pub addr: Option<String>,

    /// Enable multifunction on the guest slot when grouping related functions
    #[serde(default)]
    pub multifunction: bool,
    
    /// ROM file path (optional)
    pub romfile: Option<String>,
}

/// USB device passthrough configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDeviceConfig {
    /// Unique identifier for the USB device
    pub id: String,
    
    /// USB device specification
    #[serde(default)]
    pub host: String,

    /// USB host bus number for Proxmox-style addressing
    pub hostbus: Option<String>,

    /// USB host port path for Proxmox-style addressing
    pub hostport: Option<String>,
    
    /// USB controller bus (optional)
    pub bus: Option<String>,
    
    /// USB controller port (optional)
    pub port: Option<String>,
}

/// XHCI controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XhciControllerConfig {
    /// Unique identifier for the controller
    pub id: String,

    /// Number of USB2 ports
    pub p2: Option<u8>,

    /// Number of USB3 ports
    pub p3: Option<u8>,

    /// Parent bus placement
    pub bus: Option<String>,

    /// Address on the selected bus
    pub addr: Option<String>,
}

/// SPICE display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceConfig {
    /// Enable SPICE display
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// SPICE server port
    #[serde(default = "default_spice_port")]
    pub port: u16,
    
    /// SPICE server address
    #[serde(default = "default_spice_addr")]
    pub addr: String,
    
    /// Disable ticketing (password authentication)
    #[serde(default)]
    pub disable_ticketing: bool,
    
    /// Enable SPICE audio
    #[serde(default)]
    pub audio: bool,
    
    /// Enable vdagent (clipboard sharing)
    #[serde(default = "default_true")]
    pub vdagent: bool,
}

fn default_spice_port() -> u16 {
    5900
}

fn default_spice_addr() -> String {
    "127.0.0.1".to_string()
}

/// Audio device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceConfig {
    /// QEMU audio device type
    pub r#type: String,

    /// Unique identifier for the audio device
    pub id: String,

    /// Bus placement for the device
    pub bus: Option<String>,

    /// Address on the selected bus
    pub addr: Option<String>,

    /// Codec address on the parent HDA controller
    pub cad: Option<u8>,

    /// Backend ID used by codec devices
    pub audiodev: Option<String>,
}

/// Input device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDeviceConfig {
    /// QEMU input device type
    pub r#type: String,
}

/// Looking Glass shared memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvshmemConfig {
    /// Enable Looking Glass shared memory
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Shared memory size in MiB
    #[serde(default = "default_ivshmem_size")]
    pub size: u32,
    
    /// Shared memory device vectors
    #[serde(default = "default_ivshmem_vectors")]
    pub vectors: u32,
    
    /// Shared memory device ID
    #[serde(default = "default_ivshmem_id")]
    pub id: String,

    /// Bus placement for the ivshmem device
    pub bus: Option<String>,

    /// Shared memory file path
    #[serde(default = "default_ivshmem_mem_path")]
    pub mem_path: String,
}

fn default_ivshmem_size() -> u32 {
    32
}

fn default_ivshmem_vectors() -> u32 {
    1
}

fn default_ivshmem_id() -> String {
    "ivshmem0".to_string()
}

fn default_ivshmem_mem_path() -> String {
    "/dev/kvmfr0".to_string()
}

/// SCSI controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScsiControllerConfig {
    /// Unique identifier for the controller
    pub id: String,
    
    /// Controller type (pvscsi, virtio-scsi, lsi, etc.)
    #[serde(default = "default_scsi_controller_type")]
    pub r#type: String,
    
    /// Number of I/O queues (for virtio-scsi)
    #[serde(default)]
    pub iothread: Option<String>,
    
    /// Maximum number of targets
    #[serde(default)]
    pub max_targets: Option<u32>,

    /// PCI/PCIe bus placement for the controller
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    pub addr: Option<String>,
}

fn default_scsi_controller_type() -> String {
    "virtio-scsi-pci".to_string()
}

/// iSCSI disk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IscsiDiskConfig {
    /// Unique identifier for the disk
    pub id: String,
    
    /// iSCSI target portal (host:port)
    pub portal: String,
    
    /// iSCSI target IQN
    pub target: String,
    
    /// LUN number
    #[serde(default)]
    pub lun: u32,
    
    /// Initiator IQN (optional)
    pub initiator: Option<String>,
    
    /// Username for authentication
    pub username: Option<String>,
    
    /// Password for authentication
    pub password: Option<String>,
    
    /// SCSI controller to attach to
    pub controller: Option<String>,
}

/// QMP monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmpConfig {
    /// Enable QMP monitoring
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Socket path for QMP connection
    pub socket_path: Option<String>,
    
    /// Socket type (unix, tcp)
    #[serde(default)]
    pub socket_type: QmpSocketType,
}

/// QMP socket type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QmpSocketType {
    #[default]
    Unix,
    Tcp,
}

/// SMBIOS system information configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbiosConfig {
    /// Manufacturer name
    pub manufacturer: Option<String>,
    
    /// Product name
    pub product: Option<String>,
    
    /// Version string
    pub version: Option<String>,
    
    /// Serial number
    pub serial: Option<String>,
    
    /// UUID for the VM
    pub uuid: Option<String>,
    
    /// SKU number
    pub sku: Option<String>,
    
    /// Family name
    pub family: Option<String>,
    
    /// VM generation ID (for Windows Server 2016+)
    pub vm_generation_id: Option<String>,
}

/// NUMA node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumaConfig {
    /// NUMA node ID
    pub id: u32,
    
    /// Memory size for this node in MiB
    pub memory: u32,
    
    /// CPU cores assigned to this node
    pub cpus: Vec<u32>,
    
    /// Host NUMA node to bind to (for host-passthrough)
    pub host_node: Option<u32>,
}

/// Hyper-V enlightenments configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypervConfig {
    /// Enable Hyper-V enlightenments
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Enable Hyper-V relaxed timing
    #[serde(default = "default_true")]
    pub relaxed: bool,
    
    /// Enable Hyper-V virtual APIC
    #[serde(default = "default_true")]
    pub vapic: bool,
    
    /// Enable Hyper-V time reference counter
    #[serde(default = "default_true")]
    pub time: bool,
    
    /// Enable Hyper-V crash MSRs
    #[serde(default)]
    pub crash: bool,
    
    /// Enable Hyper-V reset MSR
    #[serde(default)]
    pub reset: bool,
    
    /// Enable Hyper-V vendor ID spoofing
    #[serde(default)]
    pub vendor_id: Option<String>,
    
    /// Enable Hyper-V frequency MSRs
    #[serde(default)]
    pub frequencies: bool,
    
    /// Enable Hyper-V reenlightenment MSRs
    #[serde(default)]
    pub reenlightenment: bool,
    
    /// Enable Hyper-V TLB flush
    #[serde(default)]
    pub tlbflush: bool,
    
    /// Enable Hyper-V IPI optimization
    #[serde(default)]
    pub ipi: bool,
    
    /// Enable Hyper-V spinlock retry
    #[serde(default)]
    pub spinlock_retry: Option<u32>,
}



impl VmConfig {
    /// Load configuration from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let processed_content = Self::substitute_env_vars(&content)?;
        let vm_value: serde_yaml::Value = serde_yaml::from_str(&processed_content)?;
        Self::ensure_yaml_mapping_root(&vm_value, "VM config")?;

        let profile_names = Self::extract_profile_names(&vm_value)?;
        let profile_dir = Self::resolve_profile_dir()?;

        let mut merged_value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        for profile_name in &profile_names {
            let profile_value = Self::load_profile_value(&profile_dir, profile_name)?;
            Self::merge_yaml_values(&mut merged_value, profile_value);
        }
        Self::merge_yaml_values(&mut merged_value, vm_value);

        let config: VmConfig = serde_yaml::from_value(merged_value)?;
        
        // Validate the configuration
        validation::validate_config(&config)?;
        
        Ok(config)
    }

    fn resolve_profile_dir() -> anyhow::Result<String> {
        let central_config = CentralConfig::load()?;
        Ok(central_config
            .locations
            .profile_dir
            .unwrap_or_else(|| DEFAULT_PROFILE_DIR.to_string()))
    }

    fn extract_profile_names(vm_value: &serde_yaml::Value) -> anyhow::Result<Vec<String>> {
        let serde_yaml::Value::Mapping(vm_map) = vm_value else {
            return Err(anyhow::anyhow!(
                "VM config must be a YAML mapping/object at the root"
            ));
        };

        let profiles_key = serde_yaml::Value::String("profiles".to_string());
        let Some(profiles_value) = vm_map.get(&profiles_key) else {
            return Ok(Vec::new());
        };

        match profiles_value {
            serde_yaml::Value::Sequence(items) => {
                let mut profile_names = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        serde_yaml::Value::String(name) if !name.trim().is_empty() => {
                            profile_names.push(name.to_string());
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "VM config field 'profiles' must contain non-empty string names"
                            ));
                        }
                    }
                }
                Ok(profile_names)
            }
            serde_yaml::Value::Null => Ok(Vec::new()),
            _ => Err(anyhow::anyhow!(
                "VM config field 'profiles' must be a list of profile names"
            )),
        }
    }

    fn load_profile_value(profile_dir: &str, profile_name: &str) -> anyhow::Result<serde_yaml::Value> {
        if profile_name.is_empty()
            || !profile_name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Err(anyhow::anyhow!(
                "Unknown profile name '{}': only [A-Za-z0-9_-] are allowed",
                profile_name
            ));
        }

        let profile_path = Path::new(profile_dir).join(format!("{}.yaml", profile_name));
        if !profile_path.exists() {
            return Err(anyhow::anyhow!(
                "Unknown profile '{}': profile file not found at '{}'",
                profile_name,
                profile_path.display()
            ));
        }

        let profile_content = std::fs::read_to_string(&profile_path).map_err(|err| {
            anyhow::anyhow!(
                "Failed to read profile '{}' from '{}': {}",
                profile_name,
                profile_path.display(),
                err
            )
        })?;
        let processed_content = Self::substitute_env_vars(&profile_content)?;
        let profile_value: serde_yaml::Value = serde_yaml::from_str(&processed_content).map_err(|err| {
            anyhow::anyhow!(
                "Failed to parse profile '{}' from '{}': {}",
                profile_name,
                profile_path.display(),
                err
            )
        })?;
        Self::ensure_yaml_mapping_root(
            &profile_value,
            &format!("Profile '{}'", profile_name),
        )?;
        Ok(profile_value)
    }

    fn ensure_yaml_mapping_root(value: &serde_yaml::Value, context: &str) -> anyhow::Result<()> {
        if !matches!(value, serde_yaml::Value::Mapping(_)) {
            return Err(anyhow::anyhow!(
                "{} must be a YAML mapping/object at the root",
                context
            ));
        }
        Ok(())
    }

    fn merge_yaml_values(base: &mut serde_yaml::Value, overlay: serde_yaml::Value) {
        match (base, overlay) {
            (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
                for (key, overlay_value) in overlay_map {
                    if let Some(base_value) = base_map.get_mut(&key) {
                        Self::merge_yaml_values(base_value, overlay_value);
                    } else {
                        base_map.insert(key, overlay_value);
                    }
                }
            }
            (base_value, overlay_value) => {
                *base_value = overlay_value;
            }
        }
    }
    
    /// Substitute environment variables in configuration content
    /// Supports ${VAR_NAME} and $VAR_NAME syntax
    fn substitute_env_vars(content: &str) -> anyhow::Result<String> {
        let mut result = content.to_string();
        
        // Find all ${VAR} patterns
        let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
        let mut replacements = Vec::new();
        
        for cap in re.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str();
            
            match std::env::var(var_name) {
                Ok(value) => {
                    replacements.push((full_match.as_str().to_string(), value));
                }
                Err(_) => {
                    return Err(anyhow::anyhow!("Environment variable '{}' not found", var_name));
                }
            }
        }
        
        // Apply replacements
        for (pattern, value) in replacements {
            result = result.replace(&pattern, &value);
        }
        
        // Also handle $VAR syntax (simple case)
        let re_simple = regex::Regex::new(r"\$([A-Z_][A-Z0-9_]*)").unwrap();
        let mut replacements_simple = Vec::new();
        
        for cap in re_simple.captures_iter(&result) {
            let full_match = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str();
            
            // Skip if it's part of a ${VAR} pattern that was already processed
            if result.contains(&format!("${{{}}}", var_name)) {
                continue;
            }
            
            match std::env::var(var_name) {
                Ok(value) => {
                    replacements_simple.push((full_match.as_str().to_string(), value));
                }
                Err(_) => {
                    return Err(anyhow::anyhow!("Environment variable '{}' not found", var_name));
                }
            }
        }
        
        // Apply simple replacements
        for (pattern, value) in replacements_simple {
            result = result.replace(&pattern, &value);
        }
        
        Ok(result)
    }
    
    /// Load configuration from a YAML string
    #[allow(dead_code)]
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let processed_content = Self::substitute_env_vars(content)?;
        let config: VmConfig = serde_yaml::from_str(&processed_content)?;
        
        // Validate the configuration
        validation::validate_config(&config)?;
        
        Ok(config)
    }
}

impl CentralConfig {
    /// Load central configuration from the default location or environment variable
    pub fn load() -> anyhow::Result<Self> {
        if let Ok(config_path) = std::env::var("EZKVM_CONFIG") {
            return if std::path::Path::new(&config_path).exists() {
                Self::from_file(&config_path)
            } else {
                Ok(Self::default())
            };
        }

        for config_path in DEFAULT_CENTRAL_CONFIG_PATHS {
            if std::path::Path::new(config_path).exists() {
                return Self::from_file(config_path);
            }
        }

        Ok(Self::default())
    }
    
    /// Load central configuration from a specific file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: CentralConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_central_config_load_honors_env_override() {
        let _guard = env_lock().lock().unwrap();

        let temp_path = std::env::temp_dir().join(format!(
            "ezkvm-central-config-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        std::fs::write(
            &temp_path,
            "tools:\n  swtpm: \"/custom/swtpm\"\n",
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &temp_path);
        }

        let config = CentralConfig::load().unwrap();
        assert_eq!(config.tools.swtpm.as_deref(), Some("/custom/swtpm"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_default_central_config_search_order_prefers_directory_path() {
        assert_eq!(
            DEFAULT_CENTRAL_CONFIG_PATHS,
            &["/etc/ezkvm/ezkvm.yaml", "/etc/ezkvm.yaml"]
        );
    }

    #[test]
    fn test_vm_config_from_file_merges_profiles_from_profile_dir() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("windows_11.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
boot:
  firmware: "uefi"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("gpu_passthrough.yaml"),
            r#"
devices:
  displays: []
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "test-vm"
backend: "qemu"
profiles:
  - "windows_11"
  - "gpu_passthrough"
system:
  memory: 8192
  vcpus: 8
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.name, "test-vm");
        assert_eq!(config.backend, "qemu");
        assert_eq!(config.system.architecture, "x86_64");
        assert_eq!(config.system.machine, "q35");
        assert_eq!(config.system.cpu_model, "host");
        assert_eq!(config.system.memory, 8192);
        assert_eq!(config.system.vcpus, 8);
        assert!(config.devices.displays.is_empty());

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_errors_on_missing_profile_file() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-missing");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "test-vm"
backend: "qemu"
profiles:
  - "does_not_exist"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let err = VmConfig::from_file(&vm_config_path).unwrap_err().to_string();
        assert!(err.contains("Unknown profile 'does_not_exist'"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_errors_on_non_mapping_profile_root() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-nonmap");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("bad_profile.yaml"),
            r#"
- not
- a
- mapping
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "test-vm"
backend: "qemu"
profiles:
  - "bad_profile"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let err = VmConfig::from_file(&vm_config_path).unwrap_err().to_string();
        assert!(err.contains("must be a YAML mapping/object at the root"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_applies_profiles_in_listed_order() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-order");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base_cpu.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("override_machine.yaml"),
            r#"
system:
  machine: "pc"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "order-test"
backend: "qemu"
profiles:
  - "base_cpu"
  - "override_machine"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.system.machine, "pc");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_deep_merges_nested_maps() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-deep-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base_system.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
boot:
  firmware: "uefi"
  menu: true
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("secure_boot.yaml"),
            r#"
boot:
  secure_boot: true
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "deep-merge-test"
backend: "qemu"
profiles:
  - "base_system"
  - "secure_boot"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.boot.firmware.as_deref(), Some("uefi"));
        assert!(config.boot.menu);
        assert!(config.boot.secure_boot);

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_replaces_lists_in_mvp_merge() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-list-replace");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base_system.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
  cpu_features:
    - name: "hv_relaxed"
    - name: "hv_time"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("replace_features.yaml"),
            r#"
system:
  cpu_features:
    - name: "kvm=off"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "list-replace-test"
backend: "qemu"
profiles:
  - "base_system"
  - "replace_features"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.system.cpu_features.len(), 1);
        assert_eq!(config.system.cpu_features[0].name, "kvm=off");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_vm_values_override_profile_values() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-vm-override");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("defaults.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "vm-override-test"
backend: "qemu"
profiles:
  - "defaults"
system:
  memory: 12288
  vcpus: 6
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.system.memory, 12288);
        assert_eq!(config.system.vcpus, 6);

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_still_runs_validation_after_profile_merge() {
        let _guard = env_lock().lock().unwrap();

        let root = unique_test_dir("ezkvm-profile-validation");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("invalid_arch.yaml"),
            r#"
system:
  architecture: "invalid_arch"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!(
                "locations:\n  profile_dir: \"{}\"\n",
                profile_dir.display()
            ),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "validation-test"
backend: "qemu"
profiles:
  - "invalid_arch"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let err = VmConfig::from_file(&vm_config_path).unwrap_err().to_string();
        assert!(err.contains("Unsupported architecture"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }
}