use serde::{Deserialize, Serialize};

/// Boot configuration.
/// Kept as a single schema type because these fields map directly to one YAML section
/// and are consumed together when emitting boot-related QEMU arguments.
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
