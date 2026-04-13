//! System configuration module
//!
//! Handles CPU, memory, and machine-specific configuration.

use super::SystemConfig;

impl SystemConfig {
    /// Get the QEMU system binary name for this architecture
    pub fn qemu_binary(&self) -> String {
        format!("qemu-system-{}", self.architecture)
    }
}
