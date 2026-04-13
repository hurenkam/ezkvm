//! Command-line interface for ezkvm
//!
//! Provides the main CLI commands for managing virtual machines.

mod commands;
mod execute;
pub(crate) mod runtime;
mod types;

#[cfg(test)]
mod tests;

pub use types::{
    Cli, Commands, DeviceCommands, NetworkCommands, PciCommands, StorageCommands, UsbCommands,
};

pub use execute::execute;
