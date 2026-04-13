use super::*;

#[test]
fn test_central_config_load_honors_env_override() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp_path = std::env::temp_dir().join(format!(
        "ezkvm-central-config-{}-{}.yaml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    std::fs::write(&temp_path, "tools:\n  swtpm: \"/custom/swtpm\"\n").unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &temp_path);
    }

    let config = CentralConfig::load().unwrap();
    assert_eq!(config.tools.swtpm.as_deref(), Some("/custom/swtpm"));

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_default_central_config_search_order_prefers_directory_path() {
    assert_eq!(
        DEFAULT_CENTRAL_CONFIG_PATHS,
        &["/etc/ezkvm/ezkvm.yaml", "/etc/ezkvm.yaml"]
    );
}
