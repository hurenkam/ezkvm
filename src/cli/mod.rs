//! Command-line interface types and entry points for the ezkvm CLI.
//!
//! Related documentation:
//! - src/README.md
//! - src/cli/README.md

mod help;
mod parser;

//use ezkvm::config_format::{ExportOptions, ImportOptions};
use serde::Deserialize;

pub use help::print_help;
pub use parser::CliArgs;

use crate::config_format::ProxmoxOptions;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImportAndExportOptions {
    pub name: String,
    #[serde(flatten)]
    pub options: ConfigOptions,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum ConfigOptions {
    Proxmox { pve: ProxmoxOptions },
    Qemu { qemu: QemuOptions },
    Libvirt { libvirt: LibvirtOptions },
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct QemuOptions {
    pub file: String,
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LibvirtOptions {
    pub file: String,
}

/// Describes a supported ezkvm CLI command and its structured options.
///
/// The enum mirrors the top-level command surface exposed by the binary.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", content = "options", rename_all = "kebab-case")]
pub enum CliCommand {
    Import(ImportAndExportOptions),
    Export(ImportAndExportOptions),
    ShowRuntime { name: String },
    Start { name: String },
    Stop { name: String },
    Reset { name: String },
    Shutdown { name: String },
}
