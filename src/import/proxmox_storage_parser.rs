use super::proxmox_model::{ProxmoxStorageConfig, ProxmoxStorageEntry};
use super::ImportError;
use std::collections::BTreeMap;

pub fn parse_proxmox_storage_config(input: &str) -> Result<ProxmoxStorageConfig, ImportError> {
    let mut storages = BTreeMap::new();
    let mut current: Option<ProxmoxStorageEntry> = None;

    for (line_no, raw_line) in input.lines().enumerate() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if raw_line.starts_with(char::is_whitespace) {
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

            let Some((key, value)) = trimmed.split_once(char::is_whitespace) else {
                return Err(ImportError::ParseError(format!(
                    "invalid storage.cfg property on line {}: '{}'",
                    line_no + 1,
                    raw_line
                )));
            };

            entry
                .options
                .insert(key.trim().to_string(), value.trim().to_string());
            continue;
        }

        if let Some(entry) = current.take() {
            storages.insert(entry.store_id.clone(), entry);
        }

        current = Some(parse_storage_header(trimmed, raw_line, line_no + 1)?);
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

#[cfg(test)]
mod tests {
    use super::parse_proxmox_storage_config;

    #[test]
    fn test_parse_storage_cfg() {
        let parsed = parse_proxmox_storage_config(
            r#"
            dir: local
                    path /var/lib/vz
                    content iso,vztmpl

            lvm: boot
                vgname boot
                content images,rootdir

            lvmthin: ws0
                    thinpool pool
                    vgname ws0
                    content rootdir,images

            lvmthin: vm0
                    thinpool pool
                    vgname vm0
                    content images,rootdir

            dir: bak
                    path /backup
                    content backup,iso,rootdir,images
                    shared 0
            "#,
        )
        .unwrap();

        assert_eq!(parsed.storages.len(), 5);
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
        assert_eq!(
            parsed
                .storages
                .get("vm0")
                .and_then(|entry| entry.options.get("vgname"))
                .map(String::as_str),
            Some("vm0")
        );
        assert_eq!(
            parsed
                .storages
                .get("bak")
                .and_then(|entry| entry.options.get("shared"))
                .map(String::as_str),
            Some("0")
        );
    }

    #[test]
    fn test_rejects_property_before_header() {
        let err = parse_proxmox_storage_config("    path /var/lib/vz").unwrap_err();
        let msg = format!("{:?}", err);
        assert!(msg.contains("indented before any storage header"));
    }
}