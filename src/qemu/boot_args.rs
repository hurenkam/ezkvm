use super::{QemuManager, types::QemuArgs};

impl QemuManager {
    /// Build boot-related arguments
    pub(super) fn build_boot_args(&self) -> QemuArgs {
        let mut args = QemuArgs::new();

        // Boot order and related boot UI options
        if !self.config.boot.boot_order.is_empty()
            || self.config.boot.menu
            || self.config.boot.strict
            || self.config.boot.reboot_timeout.is_some()
            || self.config.boot.splash.is_some()
        {
            args.push_str("-boot");
            let mut parts = Vec::new();

            if !self.config.boot.boot_order.is_empty() {
                let order: Vec<String> = self
                    .config
                    .boot
                    .boot_order
                    .iter()
                    .map(|device| match device.as_str() {
                        "disk" | "hd" => "c".to_string(),
                        "cdrom" | "cd" => "d".to_string(),
                        "floppy" => "a".to_string(),
                        "network" => "n".to_string(),
                        _ => device.clone(),
                    })
                    .collect();
                parts.push(format!("order={}", order.join("")));
            }

            if self.config.boot.menu {
                parts.push("menu=on".to_string());
            }

            if self.config.boot.strict {
                parts.push("strict=on".to_string());
            }

            if let Some(timeout) = self.config.boot.reboot_timeout {
                parts.push(format!("reboot-timeout={}", timeout));
            }

            if let Some(splash) = &self.config.boot.splash {
                parts.push(format!("splash={}", splash));
            }

            args.push(parts.join(","));
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

        // UEFI firmware (enhanced support)
        if let Some(firmware) = &self.config.boot.firmware
            && (firmware == "uefi" || firmware == "ovmf")
        {
            let code_path = if let Some(code) = &self.config.boot.uefi_code {
                Some(code.clone())
            } else {
                self.central_config
                    .locations
                    .ovmf_dir
                    .as_ref()
                    .map(|ovmf_dir| format!("{}/OVMF.fd", ovmf_dir))
            };
            args.add_uefi(
                code_path.as_deref(),
                self.config.boot.uefi_vars.as_deref(),
                self.config.boot.uefi_vars_size,
                self.config.boot.secure_boot,
            );
        }

        args
    }
}
