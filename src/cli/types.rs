use clap::{Parser, Subcommand, ValueEnum};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ImportOutputModeArg {
    Canonical,
    Compact,
    Debug,
}

/// ezkvm - Easy KVM virtual machine manager
#[derive(Parser)]
#[command(name = "ezkvm")]
#[command(about = "A simple KVM virtual machine manager using YAML configuration")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create and validate a VM configuration
    Create {
        /// Path to the YAML configuration file
        config: String,

        /// Validate only, don't create
        #[arg(long)]
        validate_only: bool,
    },

    /// Start a virtual machine
    Start {
        /// Path to the YAML configuration file
        config: String,

        /// Run in background (daemon mode)
        #[arg(short, long)]
        daemon: bool,

        /// Dry run - show command without executing
        #[arg(long)]
        dry_run: bool,

        /// Override runtime directory root for auxiliary sockets/logs
        #[arg(long)]
        run_dir: Option<String>,

        /// Override swtpm binary path
        #[arg(long)]
        swtpm_binary: Option<String>,

        /// Override TPM socket path
        #[arg(long)]
        tpm_socket_path: Option<String>,

        /// Override remote-viewer binary path
        #[arg(long)]
        remote_viewer_program: Option<String>,

        /// Override Looking Glass client binary path
        #[arg(long)]
        looking_glass_program: Option<String>,

        /// Override OVMF firmware directory used for code discovery
        #[arg(long)]
        ovmf_dir: Option<String>,
    },

    /// Stop a virtual machine
    Stop {
        /// Path to the YAML configuration file
        config: String,

        /// Force stop (SIGKILL)
        #[arg(short, long)]
        force: bool,
    },

    /// Kill a virtual machine forcefully
    Kill {
        /// Path to the YAML configuration file
        config: String,
    },

    /// List running virtual machines
    List,

    /// Show status of a virtual machine
    Status {
        /// Path to the YAML configuration file
        config: String,
    },

    /// Attach to VM console
    Console {
        /// Path to the YAML configuration file
        config: String,
    },

    /// Validate a configuration file
    Validate {
        /// Path to the YAML configuration file
        config: String,

        /// Print the resolved config after profile merging
        #[arg(long)]
        show_resolved_config: bool,
    },

    /// Import a Proxmox VM config into canonical ezkvm YAML
    #[command(
        after_help = "Examples:\n  ezkvm import-proxmox /etc/pve/qemu-server/108.conf --dry-run\n  ezkvm import-proxmox /etc/pve/qemu-server/108.conf --output-mode canonical --dry-run\n  ezkvm import-proxmox /etc/pve/qemu-server/108.conf --output-mode debug --dry-run"
    )]
    ImportProxmox {
        /// Path to Proxmox VM config (e.g. /etc/pve/qemu-server/100.conf)
        input: String,

        /// Optional path to Proxmox storage metadata file (e.g. /etc/pve/storage.cfg)
        #[arg(long)]
        proxmox_storage: Option<String>,

        /// Optional output path for generated canonical YAML
        #[arg(short, long)]
        output: Option<String>,

        /// Dry run: print generated YAML and do not write files
        #[arg(long)]
        dry_run: bool,

        /// Fail if mapper emits warnings
        #[arg(long)]
        strict: bool,

        /// Disable compact inline mapping style for list and deep nested items
        #[arg(long)]
        no_compact: bool,

        /// Output mode for generated YAML
        #[arg(
            long,
            value_enum,
            default_value_t = ImportOutputModeArg::Compact,
            help = "Output mode: canonical (full), compact (profile-overlay), debug (canonical + deterministic ids + source comments)"
        )]
        output_mode: ImportOutputModeArg,
    },

    /// Storage management commands
    #[command(subcommand)]
    Storage(StorageCommands),

    /// Device management commands
    #[command(subcommand)]
    Device(DeviceCommands),

    /// Network management commands
    #[command(subcommand)]
    Network(NetworkCommands),
}

#[derive(Subcommand)]
pub enum StorageCommands {
    /// Create a QCOW2 disk image
    Create {
        /// Name of the disk image
        name: String,
        /// Size in GB
        #[arg(short, long)]
        size: u32,
    },

    /// List all disk images
    List,

    /// Show disk image information
    Info {
        /// Name or path of the disk image
        disk: String,
    },

    /// Resize a disk image
    Resize {
        /// Name of the disk image
        disk: String,
        /// New size in GB
        #[arg(short, long)]
        size: u32,
    },

    /// Create a snapshot of a disk
    Snapshot {
        /// Name of the disk image
        disk: String,
        /// Snapshot name
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum DeviceCommands {
    /// List available USB devices
    Usb {
        #[command(subcommand)]
        cmd: Option<UsbCommands>,
    },

    /// List available PCI devices
    Pci {
        #[command(subcommand)]
        cmd: Option<PciCommands>,
    },
}

#[derive(Subcommand)]
pub enum UsbCommands {
    /// List USB devices
    List,
}

#[derive(Subcommand)]
pub enum PciCommands {
    /// List PCI devices suitable for passthrough
    List,
}

#[derive(Subcommand)]
pub enum NetworkCommands {
    /// Create a network bridge
    Bridge {
        /// Name of the bridge
        name: String,
    },
}
