//! QEMU command builder
//!
//! Provides a fluent interface for building QEMU commands.

#![allow(dead_code)]
#![allow(deprecated)]

use super::types::QemuArgs;
use crate::config::VmConfig;

/// Builder for QEMU commands
#[deprecated(
    since = "0.1.0",
    note = "QemuCommandBuilder is currently not used by the runtime path. Prefer QemuManager::build_command()."
)]
pub struct QemuCommandBuilder {
    args: QemuArgs,
}

impl QemuCommandBuilder {
    /// Create a new command builder
    pub fn new() -> Self {
        Self {
            args: QemuArgs::new(),
        }
    }

    /// Set VM name
    pub fn name(mut self, name: &str) -> Self {
        self.args.add_name(name);
        self
    }

    /// Build from a VM configuration
    pub fn from_config(config: &VmConfig) -> Self {
        let mut builder = Self::new();

        // Add VM name first
        builder = builder.name(&config.name);

        // Add system configuration
        builder = builder
            .machine(&config.system.machine)
            .cpu(&config.system.cpu.model)
            .memory(config.system.memory.size)
            .smp(config.system.cpu.vcpus);

        // Add CPU features
        for feature in &config.system.cpu.features {
            builder = builder.cpu_feature(feature);
        }

        // Add devices
        for drive in &config.devices.drives {
            builder = builder.drive_enhanced(
                &drive.path,
                &drive.interface,
                &drive.format,
                drive.readonly,
                drive.discard,
                drive.ssd,
                drive.controller.as_deref(),
            );
        }

        for network in &config.devices.networks {
            builder.args.extend(QemuArgs::from(network.clone()));
        }

        for display in &config.devices.displays {
            builder = builder.display(&display.r#type, display.vram);
        }

        // Add boot configuration
        if !config.system.boot.boot_order.is_empty() {
            builder = builder.boot_order(&config.system.boot.boot_order);
        }

        if let Some(kernel) = &config.system.boot.kernel {
            builder = builder.kernel(kernel);
        }

        if let Some(initrd) = &config.system.boot.initrd {
            builder = builder.initrd(initrd);
        }

        if let Some(cmdline) = &config.system.boot.cmdline {
            builder = builder.append(cmdline);
        }

        // Add options
        if config.options.enable_kvm {
            builder = builder.enable_kvm();
        }

        if config.options.daemonize {
            builder = builder.daemonize();
        }

        builder
    }

    /// Set machine type
    pub fn machine(mut self, machine: &str) -> Self {
        self.args.add_machine(machine);
        self
    }

    /// Set CPU model
    pub fn cpu(mut self, cpu: &str) -> Self {
        self.args.add_cpu(cpu);
        self
    }

    /// Add CPU feature
    pub fn cpu_feature(mut self, feature: &str) -> Self {
        self.args.add_cpu_feature(feature);
        self
    }

    /// Set memory in MiB
    pub fn memory(mut self, memory_mib: u32) -> Self {
        self.args.add_memory(memory_mib);
        self
    }

    /// Set SMP configuration
    pub fn smp(mut self, cpus: u32) -> Self {
        self.args.add_smp(cpus);
        self
    }

    /// Add a drive
    pub fn drive(mut self, path: &str, interface: &str, format: &str, readonly: bool) -> Self {
        self.args.add_drive(path, interface, format, readonly);
        self
    }

    /// Add an enhanced drive with advanced options
    #[allow(clippy::too_many_arguments)]
    pub fn drive_enhanced(
        mut self,
        path: &str,
        interface: &str,
        format: &str,
        readonly: bool,
        discard: bool,
        ssd: bool,
        controller: Option<&str>,
    ) -> Self {
        self.args
            .add_drive_enhanced(path, interface, format, readonly, discard, ssd, controller);
        self
    }

    /// Add a network device
    pub fn network(mut self, model: &str, mode: &str, mac: Option<&str>) -> Self {
        self.args.add_network(model, mode, mac);
        self
    }

    /// Add a display device
    pub fn display(mut self, display_type: &str, vram_mib: Option<u32>) -> Self {
        self.args.add_display(display_type, vram_mib);
        self
    }

    /// Set boot order
    pub fn boot_order(mut self, order: &[String]) -> Self {
        self.args.add_boot_order(order);
        self
    }

    /// Set kernel
    pub fn kernel(mut self, kernel: &str) -> Self {
        self.args.add_kernel(kernel);
        self
    }

    /// Set initrd
    pub fn initrd(mut self, initrd: &str) -> Self {
        self.args.add_initrd(initrd);
        self
    }

    /// Set kernel command line
    pub fn append(mut self, cmdline: &str) -> Self {
        self.args.add_append(cmdline);
        self
    }

    /// Enable KVM
    pub fn enable_kvm(mut self) -> Self {
        self.args.add_enable_kvm();
        self
    }

    /// Enable daemon mode
    pub fn daemonize(mut self) -> Self {
        self.args.add_daemonize();
        self
    }

    /// Build the final command arguments
    pub fn build(self) -> QemuArgs {
        self.args
    }
}

impl Default for QemuCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}
