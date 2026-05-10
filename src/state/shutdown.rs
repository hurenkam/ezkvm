use anyhow::Result;
use std::fs;
use std::path::Path;

/// Save a marker that guest shutdown has been observed and teardown is pending.
pub fn save_shutdown_marker(marker_path: &Path) -> Result<()> {
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(marker_path, b"shutdown\n")?;
    Ok(())
}

/// Check whether the shutdown marker is currently present.
pub fn shutdown_marker_exists(marker_path: &Path) -> bool {
    marker_path.exists()
}

/// Remove a shutdown marker if present.
pub fn delete_shutdown_marker(marker_path: &Path) -> Result<()> {
    if marker_path.exists() {
        fs::remove_file(marker_path)?;
    }
    Ok(())
}
