use std::path::PathBuf;

/// Disk information
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub format: String,
    pub size_gb: u32,
    pub actual_size_mb: u32,
    pub path: PathBuf,
}
