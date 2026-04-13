use super::{QemuManager, types::QemuArgs};
use anyhow::Result;

mod composition;

impl QemuManager {
    /// Generate the complete QEMU command line.
    pub fn build_command(&self) -> Result<QemuArgs> {
        let mut args = QemuArgs::new();

        self.add_base_args(&mut args);
        self.add_devices_and_boot_args(&mut args)?;
        self.add_platform_args(&mut args)?;
        self.add_monitoring_and_identity_args(&mut args);

        args.extend(self.build_option_args()?);

        if self.config.options.enable_kvm {
            args.push_str("-enable-kvm");
        }
        if self.config.options.daemonize {
            args.push_str("-daemonize");
        }

        Ok(args)
    }

    /// Build option-related arguments
    fn build_option_args(&self) -> Result<QemuArgs> {
        let mut args = QemuArgs::new();

        let pid_file = crate::state::get_pid_file_at(
            &self.config.name,
            self.config.options.pid_file.as_deref(),
        )?;
        args.add_pidfile(&pid_file.to_string_lossy());

        if self.config.options.nodefaults {
            args.add_nodefaults();
        }

        if self.has_primary_passthrough_gpu() {
            args.add_vga_none();
            args.add_nographic();
        }

        for global in &self.config.options.global_options {
            args.add_global(global);
        }

        if let Some(rtc) = &self.config.options.rtc {
            args.add_rtc(rtc.base.as_deref(), rtc.driftfix.as_deref());
        }

        Ok(args)
    }
}
