use super::helpers::is_enabled;
use super::super::model::{ProxmoxDiskEntry, ProxmoxStorageConfig};
use super::{DriveConfig, MappingWarning, SataControllerConfig, ScsiControllerConfig};
use std::collections::BTreeMap;

pub(super) fn map_scsi_controllers(
    scalars: &BTreeMap<String, String>,
    disks: &[ProxmoxDiskEntry],
    warnings: &mut Vec<MappingWarning>,
) -> Vec<ScsiControllerConfig> {
    let has_scsi_disks = disks.iter().any(|d| d.bus == "scsi");
    let scsihw = scalars.get("scsihw").map(String::as_str);

    if !has_scsi_disks && scsihw.is_none() {
        return Vec::new();
    }

    let controller_type = match scsihw.unwrap_or("virtio-scsi-single") {
        "virtio-scsi-single" | "virtio-scsi-pci" => "virtio-scsi-pci".to_string(),
        "pvscsi" => "pvscsi".to_string(),
        "lsi" => "lsi".to_string(),
        "lsi53c810" | "lsi53c895a" => "lsi53c895a".to_string(),
        "megasas" => "megasas".to_string(),
        "megasas-gen2" => "megasas-gen2".to_string(),
        unknown => {
            warnings.push(MappingWarning {
                source_field: "scsihw".to_string(),
                message: format!(
                    "unsupported scsihw '{}' mapped to 'virtio-scsi-pci'",
                    unknown
                ),
            });
            "virtio-scsi-pci".to_string()
        }
    };

    vec![ScsiControllerConfig {
        id: String::new(),
        r#type: controller_type,
        iothread: None,
        max_targets: None,
        bus: None,
        addr: None,
    }]
}

pub(super) fn map_drive(
    disk: &ProxmoxDiskEntry,
    storage_config: Option<&ProxmoxStorageConfig>,
) -> DriveConfig {
    let is_cdrom = disk
        .options
        .get("media")
        .map(|v| v == "cdrom")
        .unwrap_or(false)
        || disk.source == "none";

    let path = if is_cdrom && disk.source == "none" {
        String::new()
    } else {
        resolve_volume_reference(&disk.source, storage_config)
            .unwrap_or_else(|| disk.source.clone())
    };

    let format = disk.options.get("format").cloned().unwrap_or_else(|| {
        if is_cdrom || path.starts_with("/dev/") {
            "raw".to_string()
        } else {
            "qcow2".to_string()
        }
    });

    let discard = is_enabled(disk.options.get("discard"));
    let ssd = is_enabled(disk.options.get("ssd"));
    let readonly = is_enabled(disk.options.get("readonly")) || is_enabled(disk.options.get("ro"));

    let boot_index = disk.options.get("boot").and_then(|v| v.parse::<u32>().ok());

    let is_block_device = path.starts_with("/dev/");
    let cache = disk.options.get("cache").cloned().or_else(|| {
        if is_block_device {
            Some("none".to_string())
        } else {
            None
        }
    });
    let aio = disk.options.get("aio").cloned().or_else(|| {
        if is_block_device {
            Some("io_uring".to_string())
        } else {
            None
        }
    });
    let detect_zeroes = disk
        .options
        .get("detect_zeroes")
        .cloned()
        .or_else(|| disk.options.get("detect-zeroes").cloned())
        .or_else(|| {
            if is_block_device {
                Some("unmap".to_string())
            } else {
                None
            }
        });

    DriveConfig {
        id: String::new(),
        path,
        interface: disk.bus.clone(),
        r#type: if is_cdrom {
            "cdrom".to_string()
        } else {
            "disk".to_string()
        },
        format,
        readonly,
        discard,
        ssd,
        cache,
        aio,
        detect_zeroes,
        controller: if disk.bus == "scsi" {
            Some("scsihw0".to_string())
        } else {
            None
        },
        boot_index,
        scsi_id: if disk.bus == "scsi" {
            Some(disk.index as u32)
        } else {
            None
        },
        rotation_rate: if ssd && disk.bus == "scsi" {
            Some(1)
        } else {
            None
        },
        bus: if disk.bus == "sata" {
            Some(format!("sata0.{}", disk.index))
        } else {
            None
        },
        unit: if disk.bus == "sata" { Some(0) } else { None },
    }
}

pub(super) fn map_sata_controllers(disks: &[ProxmoxDiskEntry]) -> Vec<SataControllerConfig> {
    let has_sata_disks = disks.iter().any(|d| d.bus == "sata");
    if !has_sata_disks {
        return Vec::new();
    }

    vec![SataControllerConfig {
        id: String::new(),
        r#type: "ahci".to_string(),
        bus: None,
        addr: None,
    }]
}

pub(super) fn resolve_volume_reference(
    source: &str,
    storage_config: Option<&ProxmoxStorageConfig>,
) -> Option<String> {
    if source.starts_with('/') {
        return Some(source.to_string());
    }

    let (store_id, volume) = source.split_once(':')?;
    let storage = storage_config?.storages.get(store_id)?;

    match storage.storage_type.as_str() {
        "dir" => storage
            .options
            .get("path")
            .map(|base_path| resolve_dir_volume(base_path, volume)),
        "lvm" | "lvmthin" => storage.options.get("vgname").and_then(|vgname| {
            if volume.contains('/') {
                None
            } else {
                Some(format!("/dev/{}/{}", vgname, volume))
            }
        }),
        "zfspool" => storage
            .options
            .get("pool")
            .map(|pool| resolve_zfspool_volume(pool, volume)),
        _ => None,
    }
}

fn resolve_zfspool_volume(pool: &str, volume: &str) -> String {
    let trimmed_pool = pool.trim_matches('/');
    let trimmed_volume = volume.trim_start_matches('/');
    format!("/dev/zvol/{}/{}", trimmed_pool, trimmed_volume)
}

fn resolve_dir_volume(base_path: &str, volume: &str) -> String {
    let trimmed_base = base_path.trim_end_matches('/');

    if volume.starts_with('/') {
        return volume.to_string();
    }

    if volume.starts_with("images/")
        || volume.starts_with("iso/")
        || volume.starts_with("vztmpl/")
        || volume.starts_with("backup/")
        || volume.starts_with("snippets/")
        || volume.starts_with("template/")
        || volume.starts_with("rootdir/")
    {
        return format!("{}/{}", trimmed_base, volume);
    }

    if let Some((prefix, _)) = volume.split_once('/')
        && prefix.chars().all(|ch| ch.is_ascii_digit())
    {
        return format!("{}/images/{}", trimmed_base, volume);
    }

    format!("{}/{}", trimmed_base, volume)
}
