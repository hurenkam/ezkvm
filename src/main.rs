//! ezkvm - Easy KVM virtual machine manager
//!
//! A simple alternative to libvirt and virt-manager that uses YAML
//! configuration files to manage QEMU/KVM virtual machines.

mod cli;
mod config;
mod device;
mod import;
mod logging;
mod network;
mod qemu;
mod state;
mod storage;

#[cfg(test)]
mod test_support;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    logging::init();
    let cli = cli::Cli::parse();

    cli::execute(cli).await
}
