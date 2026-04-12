use serde::{Deserialize, Serialize};

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
