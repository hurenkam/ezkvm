//! Integration tests for ezkvm functionality

use ezkvm::config::{CentralConfig, VmConfig};
use ezkvm::qemu::QemuManager;
use std::fs;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[path = "integration/capability_resolution_matrix.rs"]
mod capability_resolution_matrix;
#[path = "integration/command_regression.rs"]
mod command_regression;
#[path = "integration/parsing.rs"]
mod parsing;
#[path = "integration/profile_compat.rs"]
mod profile_compat;
#[path = "integration/proxmox_import.rs"]
mod proxmox_import;
#[path = "integration/proxmox_import_output_modes.rs"]
mod proxmox_import_output_modes;
#[path = "integration/proxmox_import_profiles.rs"]
mod proxmox_import_profiles;
#[path = "integration/proxmox_import_wakiza.rs"]
mod proxmox_import_wakiza;
#[path = "integration/runtime_preflight.rs"]
mod runtime_preflight;
#[path = "integration/wakiza.rs"]
mod wakiza;
