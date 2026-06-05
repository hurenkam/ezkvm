//! Command-line interface types and entry points for the ezkvm CLI.
//!
//! Related documentation:
//! - src/README.md
//! - src/cli/README.md

mod help;
mod parser;

use ezkvm::config_format::{ExportOptions, ImportOptions};
use serde::Deserialize;

/// Re-export of the CLI help output function.
pub use help::print_help;
/// Re-export of the CLI argument parser.
pub use parser::parse_cli_options;

/// Describes a supported ezkvm CLI command and its structured options.
///
/// The enum mirrors the top-level command surface exposed by the binary.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", content = "options", rename_all = "kebab-case")]
pub enum CliCommand {
    /// Imports a source VM configuration into ezkvm's runtime model.
    Import {
        /// Import options for the selected source format.
        input: ImportOptions,
    },
    /// Exports a runtime model into a target representation.
    Export {
        /// Export options for the selected destination format.
        output: ExportOptions,
    },
    /// Imports a source configuration and exports the resulting runtime view.
    Convert {
        /// Import options for the source configuration.
        input: ImportOptions,
        /// Export options for the target representation.
        output: ExportOptions,
    },
    /// Prints the runtime representation for a named VM.
    ShowRuntime {
        /// Name of the VM whose runtime should be displayed.
        name: String,
    },
    /// Starts a named VM.
    Start {
        /// Name of the VM to start.
        name: String,
    },
    /// Stops a named VM.
    Stop {
        /// Name of the VM to stop.
        name: String,
    },
    /// Resets a named VM.
    Reset {
        /// Name of the VM to reset.
        name: String,
    },
    /// Shuts down a named VM.
    Shutdown {
        /// Name of the VM to shut down.
        name: String,
    },
}
