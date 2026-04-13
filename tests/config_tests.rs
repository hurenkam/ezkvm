//! Unit tests for configuration parsing and validation

use ezkvm::config::VmConfig;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[path = "config/config_boot_and_qemu.rs"]
mod config_boot_and_qemu;
#[path = "config/config_device_serial_display.rs"]
mod config_device_serial_display;
#[path = "config/config_parsing_env.rs"]
mod config_parsing_env;
#[path = "config/config_spice_usb_input.rs"]
mod config_spice_usb_input;
