// Phase 3 Plan 03-04: ProxmoxStorageConf and FromStr

use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum StorageType {
    Dir,
    LvmThin,
    Lvm,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxStorageEntry {
    pub storage_type: StorageType,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProxmoxStorageConf {
    pub entries: BTreeMap<String, ProxmoxStorageEntry>,
}

enum StorageParserState {
    Idle,
    Building { name: String, entry: ProxmoxStorageEntry },
}

impl FromStr for ProxmoxStorageConf {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut conf = ProxmoxStorageConf::default();
        let mut state = StorageParserState::Idle;

        for line in s.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('\t') {
                // Property line: strip leading tab, split on first space
                if let StorageParserState::Building { ref mut entry, .. } = state {
                    let stripped = &line[1..];
                    if let Some((key, value)) = stripped.split_once(' ') {
                        entry.properties.insert(key.to_string(), value.to_string());
                    }
                }
            } else {
                // Header line: finalize previous entry, start new one
                if let StorageParserState::Building { name, entry } =
                    std::mem::replace(&mut state, StorageParserState::Idle)
                {
                    conf.entries.insert(name, entry);
                }

                if let Some((type_str, name)) = line.split_once(": ") {
                    let storage_type = match type_str {
                        "dir" => StorageType::Dir,
                        "lvmthin" => StorageType::LvmThin,
                        "lvm" => StorageType::Lvm,
                        other => StorageType::Unknown(other.to_string()),
                    };
                    state = StorageParserState::Building {
                        name: name.to_string(),
                        entry: ProxmoxStorageEntry {
                            storage_type,
                            properties: BTreeMap::new(),
                        },
                    };
                }
            }
        }

        // Finalize last entry
        if let StorageParserState::Building { name, entry } = state {
            conf.entries.insert(name, entry);
        }

        Ok(conf)
    }
}

pub struct StorageResolver<'a> {
    storage_conf: &'a ProxmoxStorageConf,
    vmid: u32,
}

impl<'a> StorageResolver<'a> {
    pub fn new(storage_conf: &'a ProxmoxStorageConf, vmid: u32) -> Self {
        StorageResolver { storage_conf, vmid }
    }

    /// Resolve a `pool:volume` reference to a host path.
    /// Returns `Err(InvalidVolumeRef)` if the string has no `:` separator.
    pub fn resolve(&self, pool_volume: &str) -> Result<String, crate::config::proxmox::error::ProxmoxImportError> {
        use crate::config::proxmox::error::ProxmoxImportError;

        let (pool, volume) = pool_volume.split_once(':').ok_or_else(|| {
            ProxmoxImportError::InvalidVolumeRef { raw: pool_volume.to_string() }
        })?;

        let entry = self.storage_conf.entries.get(pool).ok_or_else(|| {
            ProxmoxImportError::UnknownStorage { pool: pool.to_string() }
        })?;

        match &entry.storage_type {
            StorageType::LvmThin | StorageType::Lvm => {
                let vgname = entry.properties.get("vgname").ok_or_else(|| {
                    ProxmoxImportError::MissingStorageProperty {
                        pool: pool.to_string(),
                        property: "vgname".to_string(),
                    }
                })?;
                Ok(format!("/dev/{}/{}", vgname, volume))
            }
            StorageType::Dir => {
                let path = entry.properties.get("path").ok_or_else(|| {
                    ProxmoxImportError::MissingStorageProperty {
                        pool: pool.to_string(),
                        property: "path".to_string(),
                    }
                })?;
                Ok(format!("{}/images/{}/{}", path, self.vmid, volume))
            }
            StorageType::Unknown(t) => Err(ProxmoxImportError::UnsupportedStorageType {
                pool: pool.to_string(),
                storage_type: t.clone(),
            }),
        }
    }
}


mod tests {
    use super::*;

    #[test]
    fn test_parse_storage_cfg_felucia() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = std::path::Path::new(&manifest).join("input/felucia/storage.cfg");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));

        let conf = ProxmoxStorageConf::from_str(&content).unwrap();

        assert!(conf.entries.contains_key("vm1-pool"), "vm1-pool entry missing");
        assert_eq!(
            conf.entries["vm1-pool"].storage_type,
            StorageType::LvmThin,
            "vm1-pool should be LvmThin"
        );
        assert_eq!(conf.entries["vm1-pool"].properties["vgname"], "vm1");
        assert_eq!(conf.entries["vm1-pool"].properties["thinpool"], "pool");
        assert!(conf.entries.contains_key("local"), "local entry missing");
    }

    #[test]
    fn test_storage_resolver_lvmthin() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = std::path::Path::new(&manifest).join("input/felucia/storage.cfg");
        let content = std::fs::read_to_string(&path).unwrap();
        let conf = ProxmoxStorageConf::from_str(&content).unwrap();
        let resolver = StorageResolver::new(&conf, 108);
        assert_eq!(
            resolver.resolve("vm1-pool:vm-108-boot").unwrap(),
            "/dev/vm1/vm-108-boot"
        );
    }

    #[test]
    fn test_storage_resolver_dir_includes_vmid() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = std::path::Path::new(&manifest).join("input/felucia/storage.cfg");
        let content = std::fs::read_to_string(&path).unwrap();
        let conf = ProxmoxStorageConf::from_str(&content).unwrap();
        let resolver = StorageResolver::new(&conf, 108);
        let result = resolver.resolve("local:iso/virtio-win.iso").unwrap();
        assert!(result.contains("/images/108/"), "path should include vmid");
    }

    #[test]
    fn test_storage_resolver_none_returns_error() {
        let conf = ProxmoxStorageConf::default();
        let resolver = StorageResolver::new(&conf, 108);
        let err = resolver.resolve("none").unwrap_err();
        assert!(matches!(err, crate::config::proxmox::error::ProxmoxImportError::InvalidVolumeRef { .. }));
    }

    #[test]
    fn test_storage_resolver_unknown_pool_returns_error() {
        let conf = ProxmoxStorageConf::default();
        let resolver = StorageResolver::new(&conf, 108);
        let err = resolver.resolve("no-such-pool:vm-disk").unwrap_err();
        assert!(matches!(err, crate::config::proxmox::error::ProxmoxImportError::UnknownStorage { .. }));
    }
}
