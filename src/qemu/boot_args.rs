use super::{QemuManager, types::QemuArgs};
use std::path::Path;

impl QemuManager {
    /// Build boot-related arguments
    pub(super) fn build_boot_args(&self) -> QemuArgs {
        let mut args = QemuArgs::new();

        self.add_boot_menu_args(&mut args);
        self.add_kernel_boot_args(&mut args);
        self.add_uefi_args(&mut args);

        args
    }

    fn add_boot_menu_args(&self, args: &mut QemuArgs) {
        let boot = self.config.system_boot();
        if boot.boot_order.is_empty()
            && !boot.menu
            && !boot.strict
            && boot.reboot_timeout.is_none()
            && boot.splash.is_none()
        {
            return;
        }

        args.push_str("-boot");
        args.push(self.build_boot_option_value());
    }

    fn build_boot_option_value(&self) -> String {
        let boot = self.config.system_boot();
        let mut parts = Vec::new();

        if !boot.boot_order.is_empty() {
            let order: String = boot
                .boot_order
                .iter()
                .map(|device| map_boot_device(device))
                .collect();
            parts.push(format!("order={}", order));
        }
        if boot.menu {
            parts.push("menu=on".to_string());
        }
        if boot.strict {
            parts.push("strict=on".to_string());
        }
        if let Some(timeout) = boot.reboot_timeout {
            parts.push(format!("reboot-timeout={}", timeout));
        }
        if let Some(splash) = &boot.splash {
            parts.push(format!("splash={}", splash));
        }

        parts.join(",")
    }

    fn add_kernel_boot_args(&self, args: &mut QemuArgs) {
        if let Some(kernel) = &self.config.system_boot().kernel {
            args.push_str("-kernel");
            args.push(kernel.clone());
        }
        if let Some(initrd) = &self.config.system_boot().initrd {
            args.push_str("-initrd");
            args.push(initrd.clone());
        }
        if let Some(cmdline) = &self.config.system_boot().cmdline {
            args.push_str("-append");
            args.push(cmdline.clone());
        }
    }

    fn add_uefi_args(&self, args: &mut QemuArgs) {
        let Some(firmware) = &self.config.system_boot().firmware else {
            return;
        };
        if firmware != "uefi" && firmware != "ovmf" {
            return;
        }

        let code_path = self.resolve_uefi_code_path();
        args.add_uefi(
            code_path.as_deref(),
            self.config.system_boot().uefi_vars.as_deref(),
            self.config.system_boot().uefi_vars_size,
            self.config.system_boot().secure_boot,
        );
    }

    fn resolve_uefi_code_path(&self) -> Option<String> {
        self.config.system_boot().uefi_code.clone().or_else(|| {
            self.central_config
                .locations
                .ovmf_dir
                .as_ref()
                .map(|ovmf_dir| {
                    resolve_ovmf_code_from_dir(ovmf_dir, self.config.system_boot().secure_boot)
                })
        })
    }
}

fn resolve_ovmf_code_from_dir(ovmf_dir: &str, secure_boot: bool) -> String {
    let preferred_files = if secure_boot {
        ["OVMF_CODE_4M.secboot.fd", "OVMF_CODE.secboot.fd", "OVMF.fd"]
    } else {
        ["OVMF_CODE_4M.fd", "OVMF_CODE.fd", "OVMF.fd"]
    };

    for file in preferred_files {
        let candidate = format!("{}/{}", ovmf_dir, file);
        if Path::new(&candidate).exists() {
            return candidate;
        }
    }

    // Fall back to the most compatible default path even if it is missing.
    format!("{}/OVMF.fd", ovmf_dir)
}

fn map_boot_device(device: &str) -> String {
    match device {
        "disk" | "hd" => "c".to_string(),
        "cdrom" | "cd" => "d".to_string(),
        "floppy" => "a".to_string(),
        "network" => "n".to_string(),
        _ => device.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_ovmf_code_from_dir;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("ezkvm-{}-{}", label, nanos))
    }

    #[test]
    fn prefers_non_secure_ovmf_code_4m_file_when_present() {
        let dir = unique_temp_dir("ovmf-non-secure");
        std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
        let file = dir.join("OVMF_CODE_4M.fd");
        std::fs::write(&file, b"mock").expect("mock firmware file should be writable");

        let resolved = resolve_ovmf_code_from_dir(&dir.to_string_lossy(), false);
        assert_eq!(resolved, file.to_string_lossy());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefers_secure_ovmf_code_4m_file_when_present() {
        let dir = unique_temp_dir("ovmf-secure");
        std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
        let file = dir.join("OVMF_CODE_4M.secboot.fd");
        std::fs::write(&file, b"mock").expect("mock firmware file should be writable");

        let resolved = resolve_ovmf_code_from_dir(&dir.to_string_lossy(), true);
        assert_eq!(resolved, file.to_string_lossy());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
