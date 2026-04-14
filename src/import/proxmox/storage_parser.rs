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
}
