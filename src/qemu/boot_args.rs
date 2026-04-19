use super::firmware_locator::{
    CentralFirmwareCapabilityResolver, FirmwareCapabilityResolver, resolve_ovmf_code_from_dir,
};
use super::{QemuManager, types::QemuArgs};

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
        let secure_boot = self.config.system_boot().secure_boot;

        // 1. CLI --ovmf-dir override: use that directory directly (always yields a path).
        if let Some(override_path) = self
            .runtime_overrides
            .ovmf_dir
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            return Some(resolve_ovmf_code_from_dir(override_path, secure_boot));
        }

        // 2. VM-level explicit firmware code path.
        if let Some(explicit) = self.config.system_boot().uefi_code.as_deref() {
            return Some(explicit.to_string());
        }

        // 3. Multi-dir resolver: central config dirs + platform defaults.
        CentralFirmwareCapabilityResolver::new(&self.central_config, &self.runtime_overrides)
            .resolve_ovmf_code(secure_boot)
    }
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
    use crate::config::{CentralConfig, LocationsConfig, RuntimeCliOverrides, VmConfig};
    use crate::qemu::QemuManager;
    use crate::qemu::resolve_ovmf_code_from_dir;
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

    fn base_uefi_vm() -> VmConfig {
        VmConfig::from_str(
            r#"
name: "uefi-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    vcpus: 1
    model: "host"
  boot:
    firmware: "uefi"
"#,
        )
        .expect("vm config should parse")
    }

    fn extract_uefi_code_arg(manager: &QemuManager) -> Option<String> {
        let args = manager.build_boot_args();
        args.iter()
            .find(|arg| arg.contains("if=pflash,unit=0,format=raw,readonly=on,file="))
            .map(|arg| arg.to_string())
    }

    #[test]
    fn uefi_precedence_prefers_cli_ovmf_dir_over_vm_and_central() {
        let mut vm = base_uefi_vm();
        vm.system.boot.uefi_code = Some("/vm-local/OVMF.fd".to_string());

        let central = CentralConfig {
            locations: crate::config::LocationsConfig {
                ovmf_dir: Some("/central/ovmf".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let manager = QemuManager::new_with_overrides(
            vm,
            central,
            RuntimeCliOverrides {
                ovmf_dir: Some("/cli/ovmf".to_string()),
                ..Default::default()
            },
        );

        let arg = extract_uefi_code_arg(&manager).expect("uefi code arg should be present");
        assert!(arg.contains("file=/cli/ovmf/OVMF.fd"));
    }

    #[test]
    fn uefi_precedence_prefers_vm_over_central_when_no_cli_override() {
        let mut vm = base_uefi_vm();
        vm.system.boot.uefi_code = Some("/vm-local/OVMF.fd".to_string());

        let central = CentralConfig {
            locations: crate::config::LocationsConfig {
                ovmf_dir: Some("/central/ovmf".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let manager = QemuManager::new_with_overrides(vm, central, RuntimeCliOverrides::default());
        let arg = extract_uefi_code_arg(&manager).expect("uefi code arg should be present");
        assert!(arg.contains("file=/vm-local/OVMF.fd"));
    }

    #[test]
    fn uefi_precedence_uses_central_ovmf_dir_before_builtin_fallback() {
        let dir = unique_temp_dir("central-ovmf");
        std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
        let file = dir.join("OVMF_CODE_4M.fd");
        std::fs::write(&file, b"mock").expect("mock firmware file should be writable");

        let vm = base_uefi_vm();
        let central = CentralConfig {
            locations: LocationsConfig {
                ovmf_dir: Some(dir.to_string_lossy().into_owned()),
                ..Default::default()
            },
            ..Default::default()
        };

        let manager = QemuManager::new_with_overrides(vm, central, RuntimeCliOverrides::default());
        let arg = extract_uefi_code_arg(&manager).expect("uefi code arg should be present");
        assert!(
            arg.contains(&file.to_string_lossy().into_owned()),
            "expected '{}' in arg '{}'",
            file.display(),
            arg
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn uefi_precedence_falls_back_to_builtin_when_no_sources_defined() {
        let vm = base_uefi_vm();
        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides::default(),
        );

        // When no sources are configured, the firmware arg must still be emitted.
        // The actual path comes from platform defaults (if OVMF is installed)
        // or the hardcoded add_uefi fallback — either way a path must be present.
        let arg = extract_uefi_code_arg(&manager).expect("uefi code arg should be present");
        assert!(
            arg.contains("file=/usr/share/"),
            "expected a /usr/share/ path, got: {}",
            arg
        );
    }
}
