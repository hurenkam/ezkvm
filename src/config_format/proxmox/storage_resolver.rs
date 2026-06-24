//! Shared Proxmox storage token/path resolver.
//!
//! This module centralizes conversion between:
//! - Proxmox storage tokens (for example `vm1-pool:vm-108-disk-0`)
//! - Canonical storage resources (`StorageResource::{File, BlockDevice}`)
//!
//! The resolver is backed by `storage.cfg` data and is reused by both import
//! and export paths.

use std::collections::BTreeMap;

use crate::runtime_model::StorageResource;

/// Parsed subset of a Proxmox `storage.cfg` needed for token/path resolution.
#[derive(Debug, Clone, Default)]
pub struct ProxmoxStorageConfig {
    storages: BTreeMap<String, StorageEntry>,
}

#[derive(Debug, Clone)]
enum StorageEntry {
    Dir { path: String },
    Lvm { vgname: String },
    LvmThin { vgname: String },
}

impl ProxmoxStorageConfig {
    /// Parses `storage.cfg` text into a lookup table.
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut storages: BTreeMap<String, StorageEntry> = BTreeMap::new();

        let mut current_type: Option<String> = None;
        let mut current_id: Option<String> = None;
        let mut current_props: BTreeMap<String, String> = BTreeMap::new();

        for (index, raw_line) in source.lines().enumerate() {
            let line_no = index + 1;
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if !raw_line.starts_with(' ') && !raw_line.starts_with('\t') {
                if let (Some(kind), Some(id)) = (current_type.take(), current_id.take()) {
                    let entry = build_entry(&kind, &id, &current_props)?;
                    storages.insert(id, entry);
                    current_props.clear();
                }

                let Some((kind, id)) = line.split_once(':') else {
                    return Err(format!(
                        "storage.cfg line {line_no}: invalid header '{line}', expected '<type>: <id>'"
                    ));
                };

                let kind = kind.trim().to_string();
                let id = id.trim().to_string();
                if kind.is_empty() || id.is_empty() {
                    return Err(format!(
                        "storage.cfg line {line_no}: empty storage type or id in '{line}'"
                    ));
                }

                current_type = Some(kind);
                current_id = Some(id);
                continue;
            }

            let Some((key, value)) = line.split_once(' ') else {
                return Err(format!(
                    "storage.cfg line {line_no}: invalid property '{line}', expected '<key> <value>'"
                ));
            };

            current_props.insert(key.trim().to_string(), value.trim().to_string());
        }

        if let (Some(kind), Some(id)) = (current_type.take(), current_id.take()) {
            let entry = build_entry(&kind, &id, &current_props)?;
            storages.insert(id, entry);
        }

        Ok(Self { storages })
    }

    /// Resolve a Proxmox disk token to a canonical storage resource.
    ///
    /// Supports:
    /// - absolute path tokens (`/path/to/file`) as file resources
    /// - `<storage_id>:<volume>` tokens resolved via parsed `storage.cfg`
    pub fn token_to_resource(&self, token: &str) -> Option<StorageResource> {
        let token = token.trim();
        if token.is_empty() || token == "none" {
            return None;
        }

        if token.starts_with('/') {
            return Some(StorageResource::File {
                file: token.to_string(),
            });
        }

        let (storage_id, volume) = token.split_once(':')?;
        let storage_id = storage_id.trim();
        let volume = volume.trim();

        let entry = self.storages.get(storage_id)?;
        match entry {
            StorageEntry::Dir { path } => Some(StorageResource::File {
                file: format!("{}/{}", path.trim_end_matches('/'), volume),
            }),
            StorageEntry::Lvm { vgname } | StorageEntry::LvmThin { vgname } => {
                Some(StorageResource::BlockDevice {
                    block_device: format!("/dev/{vgname}/{volume}"),
                })
            }
        }
    }

    /// Resolve a canonical storage resource to a Proxmox token.
    ///
    /// If no matching storage id can be found, falls back to absolute paths.
    pub fn resource_to_token(&self, resource: &StorageResource) -> String {
        match resource {
            StorageResource::File { file } => {
                if let Some(token) = self.find_dir_token(file) {
                    token
                } else {
                    file.clone()
                }
            }
            StorageResource::BlockDevice { block_device } => {
                if let Some(token) = self.find_block_token(block_device) {
                    token
                } else {
                    block_device.clone()
                }
            }
        }
    }

    fn find_dir_token(&self, file_path: &str) -> Option<String> {
        let mut best: Option<(usize, String)> = None;

        for (storage_id, entry) in &self.storages {
            let StorageEntry::Dir { path } = entry else {
                continue;
            };
            let normalized = path.trim_end_matches('/');
            if file_path == normalized {
                return None;
            }

            let prefix = format!("{normalized}/");
            if let Some(rest) = file_path.strip_prefix(&prefix) {
                let candidate = format!("{storage_id}:{rest}");
                let score = prefix.len();
                match &best {
                    Some((best_score, _)) if *best_score >= score => {}
                    _ => best = Some((score, candidate)),
                }
            }
        }

        best.map(|(_, token)| token)
    }

    fn find_block_token(&self, block_device_path: &str) -> Option<String> {
        let marker = "/dev/";
        let rest = block_device_path.strip_prefix(marker)?;
        let (vgname, volume) = rest.split_once('/')?;

        for (storage_id, entry) in &self.storages {
            match entry {
                StorageEntry::Lvm { vgname: vg } | StorageEntry::LvmThin { vgname: vg }
                    if vg == vgname =>
                {
                    return Some(format!("{storage_id}:{volume}"));
                }
                _ => {}
            }
        }

        None
    }
}

fn build_entry(
    kind: &str,
    id: &str,
    props: &BTreeMap<String, String>,
) -> Result<StorageEntry, String> {
    match kind {
        "dir" => {
            let path = props
                .get("path")
                .cloned()
                .ok_or_else(|| format!("storage '{id}' (dir) missing required property 'path'"))?;
            Ok(StorageEntry::Dir { path })
        }
        "lvm" => {
            let vgname = props.get("vgname").cloned().ok_or_else(|| {
                format!("storage '{id}' (lvm) missing required property 'vgname'")
            })?;
            Ok(StorageEntry::Lvm { vgname })
        }
        "lvmthin" => {
            let vgname = props.get("vgname").cloned().ok_or_else(|| {
                format!("storage '{id}' (lvmthin) missing required property 'vgname'")
            })?;
            Ok(StorageEntry::LvmThin { vgname })
        }
        _ => Err(format!(
            "unsupported storage type '{kind}' for storage '{id}'"
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime_model::StorageResource;

    use super::ProxmoxStorageConfig;

    fn sample_cfg() -> &'static str {
        r#"
dir: local
    path /var/lib/vz

lvm: vm0
    vgname vm0

lvmthin: vm1-pool
    vgname vm1
"#
    }

    #[test]
    fn parses_storage_cfg() {
        let parsed = ProxmoxStorageConfig::parse(sample_cfg());
        assert!(parsed.is_ok());
    }

    #[test]
    fn token_to_resource_for_lvmthin() {
        let cfg = ProxmoxStorageConfig::parse(sample_cfg()).unwrap();
        let resolved = cfg.token_to_resource("vm1-pool:vm-108-disk-0").unwrap();

        match resolved {
            StorageResource::BlockDevice { block_device } => {
                assert_eq!(block_device, "/dev/vm1/vm-108-disk-0");
            }
            _ => panic!("expected block device"),
        }
    }

    #[test]
    fn token_to_resource_for_dir() {
        let cfg = ProxmoxStorageConfig::parse(sample_cfg()).unwrap();
        let resolved = cfg.token_to_resource("local:iso/debian.iso").unwrap();

        match resolved {
            StorageResource::File { file } => {
                assert_eq!(file, "/var/lib/vz/iso/debian.iso");
            }
            _ => panic!("expected file"),
        }
    }

    #[test]
    fn resource_to_token_for_block_device() {
        let cfg = ProxmoxStorageConfig::parse(sample_cfg()).unwrap();
        let token = cfg.resource_to_token(&StorageResource::BlockDevice {
            block_device: "/dev/vm1/vm-200-disk-1".to_string(),
        });
        assert_eq!(token, "vm1-pool:vm-200-disk-1");
    }

    #[test]
    fn resource_to_token_for_file_path() {
        let cfg = ProxmoxStorageConfig::parse(sample_cfg()).unwrap();
        let token = cfg.resource_to_token(&StorageResource::File {
            file: "/var/lib/vz/iso/debian.iso".to_string(),
        });
        assert_eq!(token, "local:iso/debian.iso");
    }

    #[test]
    fn absolute_token_stays_file_resource() {
        let cfg = ProxmoxStorageConfig::parse(sample_cfg()).unwrap();
        let resolved = cfg.token_to_resource("/mnt/custom/image.qcow2").unwrap();

        match resolved {
            StorageResource::File { file } => {
                assert_eq!(file, "/mnt/custom/image.qcow2");
            }
            _ => panic!("expected file"),
        }
    }
}
