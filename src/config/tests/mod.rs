use super::*;
use crate::test_support::env_lock;

fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

mod central_config_schema;
mod compact_serialization;
mod controller_owned_devices;
mod machine_layout;
mod path_behavior;
mod profile_merge;
