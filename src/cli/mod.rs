mod help;
mod parser;

use ezkvm::config_format::{ExportOptions, ImportOptions};
use serde::Deserialize;

pub use help::print_help;
pub use parser::parse_cli_options;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", content = "options", rename_all = "kebab-case")]
pub enum CliCommand {
    Import {
        input: ImportOptions,
    },
    Export {
        output: ExportOptions,
    },
    Convert {
        input: ImportOptions,
        output: ExportOptions,
    },
    ShowRuntime {
        name: String,
    },
    Start {
        name: String,
    },
    Stop {
        name: String,
    },
    Reset {
        name: String,
    },
    Shutdown {
        name: String,
    },
}
