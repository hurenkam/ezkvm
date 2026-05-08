use std::sync::{Mutex, OnceLock};

pub(crate) fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) fn vm_config_from_yaml(yaml: &str) -> crate::config::VmConfig {
    crate::config::VmConfig::from_str(yaml).expect("vm config should parse")
}

pub(crate) fn qemu_manager_from_yaml(yaml: &str) -> crate::qemu::QemuManager {
    crate::qemu::QemuManager::new_with_overrides(
        vm_config_from_yaml(yaml),
        crate::config::CentralConfig::default(),
        crate::config::RuntimeCliOverrides::default(),
    )
}

pub(crate) fn built_qemu_args_from_yaml(yaml: &str) -> Vec<String> {
    qemu_manager_from_yaml(yaml)
        .build_command()
        .expect("qemu command should build")
        .build()
}
