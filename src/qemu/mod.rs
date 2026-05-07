//! QEMU integration module
//!
//! Handles QEMU command generation and process management.

pub mod args;
mod boot_args;
pub mod builder;
mod command_builder;
pub mod executor;
pub mod firmware_locator;
mod manager;
mod manager_build;
mod preflight;
pub mod process;
pub mod topology;
pub mod types;

#[allow(unused_imports)]
pub use firmware_locator::{
    CentralFirmwareCapabilityResolver, FirmwareCapabilityResolver, resolve_ovmf_code_from_dir,
};
pub use manager::QemuManager;
