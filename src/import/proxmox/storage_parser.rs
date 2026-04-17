use super::error::ImportError;
use super::model::{ProxmoxStorageConfig, ProxmoxStorageEntry};
use std::collections::BTreeMap;

pub fn parse_proxmox_storage_config(input: &str) -> Result<ProxmoxStorageConfig, ImportError> {
    let mut storages = BTreeMap::new();
    let mut current: Option<ProxmoxStorageEntry> = None;

    for (line_no, raw_line) in input.lines().enumerate() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if looks_like_storage_header(trimmed) {
            if let Some(entry) = current.take() {
                storages.insert(entry.store_id.clone(), entry);
            }
            current = Some(parse_storage_header(trimmed, raw_line, line_no + 1)?);
            continue;
        }

        let Some(entry) = current.as_mut() else {
            return Err(ImportError::ParseError(format!(
                "storage.cfg line {} is indented before any storage header",
                line_no + 1
            )));
        };

        let Some((key, value)) = split_property(trimmed) else {
            return Err(ImportError::ParseError(format!(
                "invalid storage.cfg property on line {}: '{}'",
                line_no + 1,
                raw_line
            )));
        };

        entry
            .options
            .insert(key.trim().to_string(), value.trim().to_string());
    }

    if let Some(entry) = current.take() {
        storages.insert(entry.store_id.clone(), entry);
    }

    Ok(ProxmoxStorageConfig { storages })
}

fn parse_storage_header(
    trimmed: &str,
    raw_line: &str,
    line_no: usize,
) -> Result<ProxmoxStorageEntry, ImportError> {
    let Some((storage_type, store_id)) = trimmed.split_once(':') else {
        return Err(ImportError::ParseError(format!(
            "invalid storage.cfg header on line {}: '{}'",
            line_no, raw_line
        )));
    };

    let storage_type = storage_type.trim();
    let store_id = store_id.trim();

    if storage_type.is_empty() || store_id.is_empty() {
        return Err(ImportError::ParseError(format!(
            "invalid storage.cfg header on line {}: '{}'",
            line_no, raw_line
        )));
    }

    Ok(ProxmoxStorageEntry {
        storage_type: storage_type.to_string(),
        store_id: store_id.to_string(),
        options: BTreeMap::new(),
    })
}

fn looks_like_storage_header(trimmed: &str) -> bool {
    let Some((storage_type, store_id)) = trimmed.split_once(':') else {
        return false;
    };

    !storage_type.trim().is_empty()
        && !store_id.trim().is_empty()
        && !storage_type.chars().any(char::is_whitespace)
}

fn split_property(trimmed: &str) -> Option<(&str, &str)> {
    let idx = trimmed.find(char::is_whitespace)?;
    let (k, v) = trimmed.split_at(idx);
    let v = v.trim_start();
    if k.is_empty() || v.is_empty() {
        return None;
    }
    Some((k, v))
}

#[cfg(test)]
mod tests {
    use super::parse_proxmox_storage_config;

    #[test]
    fn parses_multiple_storage_entries() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: local
                path /var/lib/vz
                content iso,vztmpl

            lvm: boot
                vgname boot
                content images,rootdir
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(parsed.storages.len(), 2);
        assert_eq!(
            parsed
                .storages
                .get("local")
                .and_then(|entry| entry.options.get("path"))
                .map(String::as_str),
            Some("/var/lib/vz")
        );
        assert_eq!(
            parsed
                .storages
                .get("boot")
                .map(|entry| entry.storage_type.as_str()),
            Some("lvm")
        );
    }

    #[test]
    fn parses_property_values_with_spaces() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: local
                path /var/lib/vz with spaces
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(
            parsed
                .storages
                .get("local")
                .and_then(|entry| entry.options.get("path"))
                .map(String::as_str),
            Some("/var/lib/vz with spaces")
        );
    }

    #[test]
    fn rejects_property_before_header() {
        let err = parse_proxmox_storage_config("path /var/lib/vz").expect_err("must fail");
        assert!(err.to_string().contains("before any storage header"));
    }

    #[test]
    fn rejects_invalid_property_without_value() {
        let err = parse_proxmox_storage_config(
            r#"
            dir: local
                path
            "#,
        )
        .expect_err("must fail");

        assert!(err.to_string().contains("invalid storage.cfg property"));
    }

    #[test]
    fn ignores_comments_and_empty_lines() {
        let parsed = parse_proxmox_storage_config(
            r#"
            # comment

            dir: local
                path /var/lib/vz
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(parsed.storages.len(), 1);
        assert!(parsed.storages.contains_key("local"));
    }

    // Comprehensive edge case tests for storage parser robustness
    #[test]
    fn parses_empty_storage_config() {
        let parsed = parse_proxmox_storage_config("").expect("empty input should parse");
        assert!(parsed.storages.is_empty());
    }

    #[test]
    fn parses_only_comments_in_storage_config() {
        let parsed = parse_proxmox_storage_config(
            r#"
            # Storage config comment
            # Another comment
            "#,
        )
        .expect("comments only should parse");
        assert!(parsed.storages.is_empty());
    }

    #[test]
    fn parses_single_storage_entry_minimal() {
        let parsed = parse_proxmox_storage_config("dir: local\n    path /var/lib/vz")
            .expect("minimal storage should parse");

        assert_eq!(parsed.storages.len(), 1);
        assert_eq!(
            parsed
                .storages
                .get("local")
                .map(|e| e.storage_type.as_str()),
            Some("dir")
        );
    }

    #[test]
    fn parses_all_storage_types() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: local
                path /var/lib/vz

            lvm: vm-vg
                vgname vm-vg

            lvmthin: pool1
                thinpool data
                vgname vm-vg

            zfspool: rpool
                pool rpool/data

            nfs: nfs-store
                server 192.168.1.100
                export /mnt/nfs

            cifs: cifs-store
                server 192.168.1.100
                share cifsshare

            cephfs: cephfs-store
                nodes node1

            glusterfs: gluster-store
                server 192.168.1.100

            iscsi: iscsi-store
                nodes node1
            "#,
        )
        .expect("all storage types should parse");

        assert_eq!(parsed.storages.len(), 9);
        assert!(parsed.storages.contains_key("local"));
        assert!(parsed.storages.contains_key("vm-vg"));
        assert!(parsed.storages.contains_key("pool1"));
        assert!(parsed.storages.contains_key("rpool"));
        assert!(parsed.storages.contains_key("nfs-store"));
        assert!(parsed.storages.contains_key("cifs-store"));
        assert!(parsed.storages.contains_key("cephfs-store"));
        assert!(parsed.storages.contains_key("gluster-store"));
        assert!(parsed.storages.contains_key("iscsi-store"));
    }

    #[test]
    fn parses_storage_with_many_properties() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: local
                path /var/lib/vz
                content iso,vztmpl,images,rootdir
                maxfiles 0
                shared 0
                disable 0
                nodes node1,node2
                prune-backups keep-newest=3
            "#,
        )
        .expect("storage with many properties should parse");

        let local = parsed
            .storages
            .get("local")
            .expect("should have local storage");
        assert_eq!(local.options.len(), 7);
        assert_eq!(
            local.options.get("content").map(String::as_str),
            Some("iso,vztmpl,images,rootdir")
        );
        assert_eq!(
            local.options.get("prune-backups").map(String::as_str),
            Some("keep-newest=3")
        );
    }

    #[test]
    fn parses_storage_id_with_underscores_and_numbers() {
        let parsed = parse_proxmox_storage_config(
            r#"
            lvm: backup_pool_001
                vgname vm-data
            "#,
        )
        .expect("storage id with underscores should parse");

        assert!(parsed.storages.contains_key("backup_pool_001"));
    }

    #[test]
    fn parses_storage_with_paths_containing_special_chars() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: backup
                path /mnt/backup/vm-images
            #,
                content images
            "#,
        )
        .expect("path with hyphens should parse");

        assert_eq!(
            parsed
                .storages
                .get("backup")
                .and_then(|e| e.options.get("path"))
                .map(String::as_str),
            Some("/mnt/backup/vm-images")
        );
    }

    #[test]
    fn parses_storage_with_property_value_containing_equals() {
        let parsed = parse_proxmox_storage_config(
            r#"
            lvmthin: pool
                thinpool data=0x123
                vgname vm-vg
            "#,
        )
        .expect("property with equals in value should parse");

        let pool = parsed.storages.get("pool").expect("should have pool");
        assert_eq!(
            pool.options.get("thinpool").map(String::as_str),
            Some("data=0x123")
        );
    }

    #[test]
    fn parses_consecutive_storage_entries() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: local1
                path /var/lib/vz1

            dir: local2
                path /var/lib/vz2

            dir: local3
                path /var/lib/vz3
            "#,
        )
        .expect("consecutive entries should parse");

        assert_eq!(parsed.storages.len(), 3);
        assert!(parsed.storages.contains_key("local1"));
        assert!(parsed.storages.contains_key("local2"));
        assert!(parsed.storages.contains_key("local3"));
    }

    #[test]
    fn storage_entries_later_override_earlier() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: local
                path /var/lib/vz1

            dir: local
                path /var/lib/vz2
            "#,
        )
        .expect("override should work");

        // The second definition should override the first
        assert_eq!(
            parsed
                .storages
                .get("local")
                .and_then(|e| e.options.get("path"))
                .map(String::as_str),
            Some("/var/lib/vz2")
        );
    }

    #[test]
    fn parses_nfs_storage_with_server_and_export() {
        let parsed = parse_proxmox_storage_config(
            r#"
            nfs: nfs-backup
                server 192.168.1.10
                export /mnt/exported
                path /var/lib/vz/nfs-backup
                nodes node1,node2
                content backup
            "#,
        )
        .expect("nfs storage should parse");

        let nfs = parsed
            .storages
            .get("nfs-backup")
            .expect("should have nfs storage");
        assert_eq!(nfs.storage_type, "nfs");
        assert_eq!(
            nfs.options.get("server").map(String::as_str),
            Some("192.168.1.10")
        );
        assert_eq!(
            nfs.options.get("export").map(String::as_str),
            Some("/mnt/exported")
        );
    }

    #[test]
    fn parses_storage_with_extra_indentation() {
        let parsed = parse_proxmox_storage_config("dir: local\n        path /var/lib/vz")
            .expect("extra indentation should still parse");

        assert_eq!(
            parsed
                .storages
                .get("local")
                .and_then(|e| e.options.get("path"))
                .map(String::as_str),
            Some("/var/lib/vz")
        );
    }
}
