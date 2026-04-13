//! Storage management for VMs
//!
//! Handles QCOW2 image creation, snapshots, and disk operations.

#![allow(dead_code)]

mod operations;
mod paths;
mod types;

#[allow(unused_imports)]
pub use operations::{
    convert_disk, create_qcow2, create_snapshot, get_disk_info, list_disks, resize_disk,
};
#[allow(unused_imports)]
pub use paths::get_storage_dir;
#[allow(unused_imports)]
pub use types::DiskInfo;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_dir_creation() {
        let storage_dir = get_storage_dir();
        assert!(storage_dir.is_ok());
        assert!(storage_dir.unwrap().exists());
    }

    #[test]
    fn test_list_disks() {
        let disks = list_disks();
        assert!(disks.is_ok());
    }
}
