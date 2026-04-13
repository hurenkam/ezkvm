use crate::qemu::types::QemuArgs;
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

impl From<SystemConfig> for QemuArgs {
    fn from(config: SystemConfig) -> Self {
        let mut args = QemuArgs::new();

        // Machine type
        args.push_str("-machine");
        let mut machine_spec = format!("type={}", config.machine);
        for option in config.machine_options {
            machine_spec.push(',');
            machine_spec.push_str(&option);
        }
        args.push(machine_spec);

        // CPU configuration
        args.push_str("-cpu");
        let mut cpu_spec = config.cpu_model.clone();
        if !config.cpu_features.is_empty() {
            let features: Vec<String> =
                config.cpu_features.iter().map(|f| f.name.clone()).collect();
            cpu_spec.push(',');
            cpu_spec.push_str(&features.join(","));
        }
        args.push(cpu_spec);

        // Memory
        args.push_str("-m");
        args.push(format!("{}M", config.memory));

        // SMP (symmetric multiprocessing)
        args.push_str("-smp");
        args.push(format!("cpus={}", config.vcpus));

        args
    }
}

impl From<DriveConfig> for QemuArgs {
    fn from(drive: DriveConfig) -> Self {
        let mut args = QemuArgs::new();
        let has_path = !drive.path.trim().is_empty();

        let drive_node_id = format!("drive-{}", drive.id);
        let needs_attached_device = drive.controller.is_some()
            || drive.boot_index.is_some()
            || drive.scsi_id.is_some()
            || drive.bus.is_some()
            || drive.unit.is_some()
            || drive.interface == "ide";

        args.push_str("-drive");

        let mut drive_parts = Vec::new();
        if has_path {
            drive_parts.push(format!("file={}", drive.path));
        }

        if needs_attached_device {
            drive_parts.push("if=none".to_string());
            drive_parts.push(format!("id={}", drive_node_id));
            if drive.r#type == "cdrom" {
                drive_parts.push("media=cdrom".to_string());
            }
        } else {
            drive_parts.push(format!("if={}", drive.interface));
        }

        if has_path {
            drive_parts.push(format!("format={}", drive.format));
        }

        if drive.readonly {
            drive_parts.push("readonly=on".to_string());
        }

        if has_path && drive.discard {
            drive_parts.push("discard=unmap".to_string());
        }

        if has_path && drive.ssd {
            drive_parts.push("ssd=on".to_string());
        }

        if has_path {
            if let Some(cache) = drive.cache.as_ref() {
                drive_parts.push(format!("cache={}", cache));
            }

            if let Some(aio) = drive.aio.as_ref() {
                drive_parts.push(format!("aio={}", aio));
            }

            if let Some(detect_zeroes) = drive.detect_zeroes.as_ref() {
                drive_parts.push(format!("detect-zeroes={}", detect_zeroes));
            }
        }

        args.push(drive_parts.join(","));

        if needs_attached_device {
            args.push_str("-device");
            let mut device_spec = match drive.interface.as_str() {
                "scsi" => format!(
                    "{},drive={},id={}",
                    if drive.r#type == "cdrom" {
                        "scsi-cd"
                    } else {
                        "scsi-hd"
                    },
                    drive_node_id,
                    drive.id
                ),
                "ide" => format!(
                    "{},drive={},id={}",
                    if drive.r#type == "cdrom" {
                        "ide-cd"
                    } else {
                        "ide-hd"
                    },
                    drive_node_id,
                    drive.id
                ),
                "virtio" => format!("virtio-blk-pci,drive={},id={}", drive_node_id, drive.id),
                "nvme" => format!("nvme,drive={},id={}", drive_node_id, drive.id),
                _ => format!(
                    "{},drive={},id={}",
                    drive.interface, drive_node_id, drive.id
                ),
            };

            let attachment_bus = drive.bus.clone().or_else(|| {
                drive
                    .controller
                    .as_ref()
                    .map(|controller| format!("{}.0", controller))
            });
            if let Some(bus) = attachment_bus {
                device_spec.push_str(&format!(",bus={}", bus));
            }

            if let Some(unit) = drive.unit {
                device_spec.push_str(&format!(",unit={}", unit));
            }

            if let Some(scsi_id) = drive.scsi_id {
                device_spec.push_str(&format!(",scsi-id={}", scsi_id));
            }

            if let Some(boot_index) = drive.boot_index {
                device_spec.push_str(&format!(",bootindex={}", boot_index));
            }

            args.push(device_spec);
        }

        args
    }
}

impl From<NetworkConfig> for QemuArgs {
    fn from(network: NetworkConfig) -> Self {
        let mut args = QemuArgs::new();

        args.push_str("-netdev");
        let netdev_spec = match network.mode.as_str() {
            "user" => format!("type=user,id={}", network.id),
            _ => format!("type={},id={}", network.mode, network.id),
        };
        args.push(netdev_spec);

        args.push_str("-device");
        let mut device_spec = format!("{},netdev={}", network.model, network.id);

        if let Some(mac) = network.mac {
            device_spec.push_str(&format!(",mac={}", mac));
        }

        if let Some(bus) = network.bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = network.addr {
            device_spec.push_str(&format!(",addr={}", addr));
        }

        if let Some(rx_queue_size) = network.rx_queue_size {
            device_spec.push_str(&format!(",rx_queue_size={}", rx_queue_size));
        }

        if let Some(tx_queue_size) = network.tx_queue_size {
            device_spec.push_str(&format!(",tx_queue_size={}", tx_queue_size));
        }

        if let Some(boot_index) = network.boot_index {
            device_spec.push_str(&format!(",bootindex={}", boot_index));
        }

        args.push(device_spec);
        args
    }
}

impl From<DisplayConfig> for QemuArgs {
    fn from(display: DisplayConfig) -> Self {
        let mut args = QemuArgs::new();

        args.push_str("-device");
        let mut device_spec = display.r#type.clone();

        if let Some(vram) = display.vram {
            match display.r#type.as_str() {
                "qxl" => device_spec.push_str(&format!(",vram_size_mb={}", vram)),
                "vmware-svga" => device_spec.push_str(&format!(",vgamem_mb={}", vram)),
                _ => device_spec.push_str(&format!(",vram={}", vram * 1024 * 1024)),
            }
        }

        args.push(device_spec);
        args
    }
}

impl From<SerialConfig> for QemuArgs {
    fn from(serial: SerialConfig) -> Self {
        let mut args = QemuArgs::new();

        match serial.r#type.as_str() {
            "pty" => {
                args.push_str("-serial");
                args.push("pty".to_string());
            }
            "stdio" => {
                args.push_str("-serial");
                args.push("stdio".to_string());
            }
            "file" => {
                args.push_str("-serial");
                args.push(format!(
                    "file:{}",
                    serial.path.unwrap_or_else(|| "/dev/null".to_string())
                ));
            }
            "socket" => {
                args.push_str("-serial");
                let host = serial.host.unwrap_or_else(|| "127.0.0.1".to_string());
                let port = serial.socket_port.unwrap_or(4444);
                let mut spec = format!("tcp:{}:{}", host, port);
                if serial.server {
                    spec.push_str(",server");
                }
                if !serial.wait {
                    spec.push_str(",nowait");
                }
                args.push(spec);
            }
            _ => {
                // Default to pty
                args.push_str("-serial");
                args.push("pty".to_string());
            }
        }

        args
    }
}
