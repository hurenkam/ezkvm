//! QEMU integration module
//!
//! Handles QEMU command generation and process management.

pub mod builder;
pub mod executor;
pub mod args;
pub mod types;

use crate::config::VmConfig;
use crate::qemu::types::QemuArgs;
use anyhow::Result;

/// Main QEMU manager
pub struct QemuManager {
    config: VmConfig,
}

impl QemuManager {
    /// Create a new QEMU manager for a VM configuration
    pub fn new(config: VmConfig) -> Self {
        Self { config }
    }
    
    /// Get a reference to the VM configuration
    pub fn config(&self) -> &VmConfig {
        &self.config
    }
    
    /// Generate the complete QEMU command line
    pub fn build_command(&self) -> Result<QemuArgs> {
        let mut args = QemuArgs::new();
        
        // Add system arguments
        args.extend(QemuArgs::from(self.config.system.clone()));
        
        // Add device arguments
        args.extend(QemuArgs::from(self.config.devices.clone()));
        
        // Add boot arguments
        args.extend(self.build_boot_args());
        
        // Add option arguments
        args.extend(self.build_option_args());
        
        // Add KVM acceleration if enabled
        if self.config.options.enable_kvm {
            args.push_str("-enable-kvm");
        }
        
        // Add daemon mode if requested
        if self.config.options.daemonize {
            args.push_str("-daemonize");
        }
        
        Ok(args)
    }
    
    /// Build boot-related arguments
    fn build_boot_args(&self) -> QemuArgs {
        let mut args = QemuArgs::new();
        
        // Boot order
        if !self.config.boot.boot_order.is_empty() {
            args.push_str("-boot");
            let order = self.config.boot.boot_order.join(",");
            args.push(format!("order={}", order));
        }
        
        // Kernel boot (if specified)
        if let Some(kernel) = &self.config.boot.kernel {
            args.push_str("-kernel");
            args.push(kernel.clone());
        }
        
        if let Some(initrd) = &self.config.boot.initrd {
            args.push_str("-initrd");
            args.push(initrd.clone());
        }
        
        if let Some(cmdline) = &self.config.boot.cmdline {
            args.push_str("-append");
            args.push(cmdline.clone());
        }
        
        args
    }
    
    /// Build option-related arguments
    fn build_option_args(&self) -> QemuArgs {
        let mut args = QemuArgs::new();
        
        // UEFI firmware
        if let Some(firmware) = &self.config.boot.firmware {
            if firmware == "uefi" {
                // Use OVMF for UEFI
                args.push_str("-bios");
                args.push("/usr/share/ovmf/OVMF.fd".to_string());
                
                // UEFI variables
                if let Some(vars) = &self.config.options.uefi_vars {
                    args.push_str("-drive");
                    args.push(format!("if=pflash,format=raw,file={},readonly=on", vars));
                }
            }
        }
        
        args
    }
    
    /// Get the QEMU binary name
    pub fn binary_name(&self) -> String {
        self.config.system.qemu_binary()
    }
}