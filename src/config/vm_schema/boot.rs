use serde::{Deserialize, Serialize};

fn is_false(value: &bool) -> bool {
    !*value
}

/// Boot configuration.
/// Kept as a single schema type because these fields map directly to one YAML section
/// and are consumed together when emitting boot-related QEMU arguments.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootConfig {
    /// Firmware type (uefi, bios, or ovmf)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware: Option<String>,

    /// Boot order (disk, cdrom, network)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boot_order: Vec<String>,

    /// Show the boot menu
    #[serde(default, skip_serializing_if = "is_false")]
    pub menu: bool,

    /// Enforce strict boot ordering
    #[serde(default, skip_serializing_if = "is_false")]
    pub strict: bool,

    /// Reboot timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reboot_timeout: Option<u32>,

    /// Splash screen image path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<String>,

    /// Kernel path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel: Option<String>,

    /// Initrd path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd: Option<String>,

    /// Kernel command line
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdline: Option<String>,

    /// UEFI firmware code path (for custom OVMF)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uefi_code: Option<String>,

    /// UEFI variables path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uefi_vars: Option<String>,

    /// Optional explicit UEFI variables drive size in bytes for pflash unit 1.
    /// Proxmox uses `size=540672` for OVMF vars even when the backing device is larger.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uefi_vars_size: Option<u64>,

    /// Enable secure boot
    #[serde(default, skip_serializing_if = "is_false")]
    pub secure_boot: bool,
}
