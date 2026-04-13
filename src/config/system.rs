//! System configuration module
//!
//! Handles CPU, memory, and machine-specific configuration.

use super::{CpuFeature, SystemConfig};

impl SystemConfig {
    /// Get the QEMU system binary name for this architecture
    pub fn qemu_binary(&self) -> String {
        format!("qemu-system-{}", self.architecture)
    }
}

impl CpuFeature {
    /// Create a new CPU feature
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Check if this is an enabled feature (+prefix)
    pub fn is_enabled(&self) -> bool {
        self.name.starts_with('+')
    }

    /// Check if this is a disabled feature (-prefix)
    pub fn is_disabled(&self) -> bool {
        self.name.starts_with('-')
    }
}
