use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;

/// Default directory for VM disk images
pub fn get_storage_dir() -> Result<PathBuf> {
    let data_home = if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data)
    } else {
        let home =
            std::env::var("HOME").map_err(|_| anyhow!("HOME environment variable not set"))?;
        PathBuf::from(home).join(".local/share")
    };

    let storage_dir = data_home.join("ezkvm/disks");
    fs::create_dir_all(&storage_dir)?;
    Ok(storage_dir)
}
