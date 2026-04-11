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

/// Central tool configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CentralConfig {
    /// Tool paths
    #[serde(default)]
    pub tools: ToolsConfig,
    
    /// Directory locations
    #[serde(default)]
    pub locations: LocationsConfig,
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
}

/// Main configuration structure for a virtual machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Name of the virtual machine
    pub name: String,
    
    /// Backend to use (currently only "qemu" is supported)
    pub backend: String,
    
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
    
    /// Path to TPM state file (for emulator backend)
    pub state_path: Option<String>,
    
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
        let config: VmConfig = serde_yaml::from_str(&processed_content)?;
        
        // Validate the configuration
        validation::validate_config(&config)?;
        
        Ok(config)
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

    #[test]
    fn test_central_config_load_honors_env_override() {
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
}