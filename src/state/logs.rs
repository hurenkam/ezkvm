use anyhow::Result;
use std::fs;

use super::get_logs_dir_at;

const DEFAULT_LOG_KEEP: usize = 10;

/// Clean up old log files (keep last N)
#[allow(dead_code)]
pub fn cleanup_old_logs(vm_name: &str) -> Result<()> {
    cleanup_old_logs_at(vm_name, None, None)
}

/// Clean up old log files in an optional custom logs directory.
pub fn cleanup_old_logs_at(
    vm_name: &str,
    custom_dir: Option<&str>,
    keep: Option<usize>,
) -> Result<()> {
    let logs_dir = get_logs_dir_at(vm_name, custom_dir)?;
    let keep = keep.unwrap_or(DEFAULT_LOG_KEEP);

    let mut log_files: Vec<_> = fs::read_dir(&logs_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "log").unwrap_or(false) {
                let metadata = entry.metadata().ok()?;
                let modified = metadata.modified().ok()?;
                Some((path, modified))
            } else {
                None
            }
        })
        .collect();

    if log_files.len() > keep {
        log_files.sort_by_key(|(_, mtime)| *mtime);
        while log_files.len() > keep {
            let (path, _) = log_files.remove(0);
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}
