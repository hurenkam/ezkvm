//! ezkvm - Easy KVM virtual machine manager
//!
//! A simple alternative to libvirt and virt-manager that uses YAML
//! configuration files to manage QEMU/KVM virtual machines.

mod cli;
mod config;
mod qemu;
mod state;
mod storage;
mod device;
mod network;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    
    cli::execute(cli).await
}
