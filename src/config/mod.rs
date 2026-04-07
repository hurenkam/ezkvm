//! Configuration module for ezkvm
//!
//! This module handles parsing and validation of YAML configuration files
//! that define virtual machine specifications.

pub mod system;
pub mod devices;
pub mod validation;

use serde::{Deserialize, Serialize};
use std::path::Path;

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
    /// Firmware type (uefi or bios)
    #[serde(default)]
    pub firmware: Option<String>,
    
    /// Boot order (disk, cdrom, network)
    #[serde(default)]
    pub boot_order: Vec<String>,
    
    /// Kernel path (optional)
    pub kernel: Option<String>,
    
    /// Initrd path (optional)
    pub initrd: Option<String>,
    
    /// Kernel command line
    pub cmdline: Option<String>,
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
}

/// Additional VM options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VmOptions {
    /// Enable KVM acceleration
    #[serde(default = "default_true")]
    pub enable_kvm: bool,
    
    /// Run in daemon mode
    #[serde(default)]
    pub daemonize: bool,
    
    /// Path to UEFI variables file
    pub uefi_vars: Option<String>,
}

fn default_true() -> bool {
    true
}

impl VmConfig {
    /// Load configuration from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: VmConfig = serde_yaml::from_str(&content)?;
        
        // Validate the configuration
        validation::validate_config(&config)?;
        
        Ok(config)
    }
    
    /// Load configuration from a YAML string
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: VmConfig = serde_yaml::from_str(content)?;
        
        // Validate the configuration
        validation::validate_config(&config)?;
        
        Ok(config)
    }
}