// Phase 3 Plan 03-01-B: split_sections() state machine
// Phase 3 Plan 03-03: parse_sub_options() and per-device tokenizers

use std::collections::BTreeMap;

use crate::config::proxmox::conf::{
    ProxmoxAudioConf, ProxmoxDiskConf, ProxmoxEfiDiskConf, ProxmoxHostPciConf,
    ProxmoxNetConf, ProxmoxSerialConf, ProxmoxTpmConf, ProxmoxUsbConf,
};
use crate::config::proxmox::error::ProxmoxParseError;

enum ParserState {
    Active,
    Snapshot(String),
}

pub fn split_sections(
    input: &str,
) -> (Vec<(String, String)>, BTreeMap<String, Vec<(String, String)>>) {
    let mut active_entries: Vec<(String, String)> = Vec::new();
    let mut snapshot_map: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut current_snapshot: Vec<(String, String)> = Vec::new();
    let mut state = ParserState::Active;

    for line in input.lines() {
        let line = line.trim_end();

        if line.is_empty() {
            continue;
        }

        // Any line starting with '#' is a comment — skip without URL-decoding
        if line.starts_with('#') {
            continue;
        }

        match &state {
            ParserState::Active => {
                if line.starts_with('[') {
                    if line.ends_with(']') {
                        let name = line[1..line.len() - 1].trim().to_string();
                        state = ParserState::Snapshot(name);
                    }
                    // malformed header — skip silently
                } else if let Some((key, value)) = line.split_once(": ") {
                    active_entries.push((key.trim().to_string(), value.to_string()));
                }
                // no ": " separator — skip silently
            }
            ParserState::Snapshot(current_name) => {
                if line.starts_with('[') {
                    if line.ends_with(']') {
                        let finished_name = current_name.clone();
                        snapshot_map.insert(finished_name, current_snapshot);
                        current_snapshot = Vec::new();
                        let new_name = line[1..line.len() - 1].trim().to_string();
                        state = ParserState::Snapshot(new_name);
                    }
                    // malformed header — skip silently
                } else if let Some((key, value)) = line.split_once(": ") {
                    current_snapshot.push((key.trim().to_string(), value.to_string()));
                }
            }
        }
    }

    // Emit final snapshot if still in Snapshot state
    if let ParserState::Snapshot(name) = state {
        snapshot_map.insert(name, current_snapshot);
    }

    (active_entries, snapshot_map)
}

pub fn parse_sub_options(raw: &str) -> Vec<(Option<String>, String)> {
    raw.split(',')
        .map(|token| match token.split_once('=') {
            Some((k, v)) => (Some(k.to_string()), v.to_string()),
            None => (None, token.to_string()),
        })
        .collect()
}

pub fn parse_disk_raw(raw: &str) -> Result<ProxmoxDiskConf, ProxmoxParseError> {
    let tokens = parse_sub_options(raw);
    let mut iter = tokens.into_iter();
    let first = iter.next().ok_or_else(|| ProxmoxParseError::InvalidSubOption {
        field: "disk".into(),
        raw: raw.into(),
    })?;
    let volume = match first {
        (None, v) => v,
        _ => {
            return Err(ProxmoxParseError::InvalidSubOption {
                field: "disk".into(),
                raw: raw.into(),
            })
        }
    };
    let options = iter
        .filter_map(|(k, v)| k.map(|key| (key, v)))
        .collect::<BTreeMap<_, _>>();
    Ok(ProxmoxDiskConf { volume, options })
}

const NET_MODELS: &[&str] = &[
    "virtio", "e1000", "e1000e", "e1000-82540em", "rtl8139", "vmxnet3",
    "i82551", "i82557b", "i82559er", "ne2k_pci", "ne2k_isa", "pcnet", "rocker",
];

pub fn parse_net_raw(raw: &str) -> Result<ProxmoxNetConf, ProxmoxParseError> {
    let tokens = parse_sub_options(raw);
    let mut model = None;
    let mut mac = None;
    let mut options = BTreeMap::new();
    for (k, v) in tokens {
        match k {
            Some(key) if NET_MODELS.contains(&key.as_str()) => {
                model = Some(key);
                mac = Some(v);
            }
            Some(key) => {
                options.insert(key, v);
            }
            None => {}
        }
    }
    match (model, mac) {
        (Some(model), Some(mac)) => Ok(ProxmoxNetConf { model, mac, options }),
        _ => Err(ProxmoxParseError::InvalidSubOption {
            field: "net".into(),
            raw: raw.into(),
        }),
    }
}

pub fn parse_hostpci_raw(raw: &str) -> Result<ProxmoxHostPciConf, ProxmoxParseError> {
    let tokens = parse_sub_options(raw);
    let mut iter = tokens.into_iter();
    let first = iter.next().ok_or_else(|| ProxmoxParseError::InvalidSubOption {
        field: "hostpci".into(),
        raw: raw.into(),
    })?;
    let bdf = match first {
        (None, v) => v,
        _ => {
            return Err(ProxmoxParseError::InvalidSubOption {
                field: "hostpci".into(),
                raw: raw.into(),
            })
        }
    };
    let options = iter
        .filter_map(|(k, v)| k.map(|key| (key, v)))
        .collect::<BTreeMap<_, _>>();
    Ok(ProxmoxHostPciConf { bdf, options })
}

pub fn parse_usb_raw(raw: &str) -> Result<ProxmoxUsbConf, ProxmoxParseError> {
    let tokens = parse_sub_options(raw);
    for (k, v) in tokens {
        if k.as_deref() == Some("host") {
            return Ok(ProxmoxUsbConf { host: v });
        }
    }
    Err(ProxmoxParseError::InvalidSubOption {
        field: "usb".into(),
        raw: raw.into(),
    })
}

pub fn parse_audio_raw(raw: &str) -> Result<ProxmoxAudioConf, ProxmoxParseError> {
    let tokens = parse_sub_options(raw);
    let mut device = None;
    let mut driver = None;
    for (k, v) in tokens {
        match k.as_deref() {
            Some("device") => device = Some(v),
            Some("driver") => driver = Some(v),
            _ => {}
        }
    }
    Ok(ProxmoxAudioConf { device, driver })
}

pub fn parse_efidisk_raw(raw: &str) -> Result<ProxmoxEfiDiskConf, ProxmoxParseError> {
    let tokens = parse_sub_options(raw);
    let mut iter = tokens.into_iter();
    let first = iter.next().ok_or_else(|| ProxmoxParseError::InvalidSubOption {
        field: "efidisk".into(),
        raw: raw.into(),
    })?;
    let volume = match first {
        (None, v) => v,
        _ => {
            return Err(ProxmoxParseError::InvalidSubOption {
                field: "efidisk".into(),
                raw: raw.into(),
            })
        }
    };
    let options = iter
        .filter_map(|(k, v)| k.map(|key| (key, v)))
        .collect::<BTreeMap<_, _>>();
    Ok(ProxmoxEfiDiskConf { volume, options })
}

pub fn parse_tpmstate_raw(raw: &str) -> Result<ProxmoxTpmConf, ProxmoxParseError> {
    let tokens = parse_sub_options(raw);
    let mut iter = tokens.into_iter();
    let first = iter.next().ok_or_else(|| ProxmoxParseError::InvalidSubOption {
        field: "tpmstate".into(),
        raw: raw.into(),
    })?;
    let volume = match first {
        (None, v) => v,
        _ => {
            return Err(ProxmoxParseError::InvalidSubOption {
                field: "tpmstate".into(),
                raw: raw.into(),
            })
        }
    };
    let options = iter
        .filter_map(|(k, v)| k.map(|key| (key, v)))
        .collect::<BTreeMap<_, _>>();
    Ok(ProxmoxTpmConf { volume, options })
}

pub fn parse_serial_raw(raw: &str) -> Result<ProxmoxSerialConf, ProxmoxParseError> {
    Ok(ProxmoxSerialConf { socket: raw.to_string() })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sections_active_has_cpu_host() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = std::path::Path::new(&manifest).join("input/felucia/108.conf");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));

        let (active, snapshots) = split_sections(&content);

        let cpu = active.iter().find(|(k, _)| k == "cpu");
        assert!(cpu.is_some(), "cpu key not found in active section");
        assert_eq!(cpu.unwrap().1, "host", "cpu should be 'host' in active section");

        assert!(
            !active.iter().any(|(k, _)| k == "snaptime"),
            "snaptime must not appear in active section"
        );
        assert!(
            !active.iter().any(|(_, v)| v.contains("x86-64-v2-AES")),
            "snapshot cpu value must not appear in active section"
        );

        assert!(snapshots.contains_key("before_lg"), "before_lg snapshot missing");
        assert!(
            snapshots.contains_key("intermediate_20251018"),
            "intermediate_20251018 snapshot missing"
        );

        let snap = &snapshots["intermediate_20251018"];
        let snap_cpu = snap.iter().find(|(k, _)| k == "cpu");
        assert!(snap_cpu.is_some(), "cpu not found in intermediate_20251018 snapshot");
        assert_eq!(
            snap_cpu.unwrap().1,
            "x86-64-v2-AES",
            "snapshot cpu should be 'x86-64-v2-AES'"
        );
    }

    #[test]
    fn test_split_sections_skips_hash_comments() {
        let input = "##args%3A some disabled config\n\
                     ##hostpci1%3A 0000%3A04%3A00.0\n\
                     ####also_disabled: value\n\
                     cpu: host\n";

        let (active, _snapshots) = split_sections(input);

        assert_eq!(active.len(), 1, "exactly one entry should survive comment stripping");
        assert_eq!(active[0].0, "cpu");
        assert_eq!(active[0].1, "host");
    }

    #[test]
    fn test_parse_sub_options_net_mac() {
        let tokens = parse_sub_options("virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1");
        assert_eq!(tokens[0], (Some("virtio".into()), "BC:24:11:3A:21:B7".into()));
        assert_eq!(tokens[1], (Some("bridge".into()), "vmbr0".into()));
        assert_eq!(tokens[2], (Some("firewall".into()), "1".into()));
    }

    #[test]
    fn test_parse_sub_options_disk_pool_volume() {
        let tokens = parse_sub_options("vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1");
        assert_eq!(tokens[0], (None, "vm1-pool:vm-108-boot".into()));
        assert_eq!(tokens[1], (Some("discard".into()), "on".into()));
    }

    #[test]
    fn test_parse_net_raw_mac_preserved() {
        let result = parse_net_raw("virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1").unwrap();
        assert_eq!(result.mac, "BC:24:11:3A:21:B7");
        assert_eq!(result.model, "virtio");
        assert_eq!(result.options["bridge"], "vmbr0");
        assert_eq!(result.options["firewall"], "1");
    }

    #[test]
    fn test_parse_hostpci_raw_bdf_preserved() {
        let result = parse_hostpci_raw("0000:03:00,pcie=1,x-vga=1").unwrap();
        assert_eq!(result.bdf, "0000:03:00");
        assert_eq!(result.options["pcie"], "1");
        assert_eq!(result.options["x-vga"], "1");
    }

    #[test]
    fn test_parse_usb_raw_port() {
        let result = parse_usb_raw("host=1-2.2").unwrap();
        assert_eq!(result.host, "1-2.2");
    }

    #[test]
    fn test_parse_usb_raw_vid_pid() {
        let result = parse_usb_raw("host=0451:16a0").unwrap();
        assert_eq!(result.host, "0451:16a0");
    }

    #[test]
    fn test_parse_efidisk_raw() {
        let result = parse_efidisk_raw(
            "vm1-pool:vm-108-efidisk,efitype=4m,pre-enrolled-keys=1,size=4M",
        )
        .unwrap();
        assert_eq!(result.volume, "vm1-pool:vm-108-efidisk");
        assert_eq!(result.options["efitype"], "4m");
        assert_eq!(result.options["pre-enrolled-keys"], "1");
        assert_eq!(result.options["size"], "4M");
    }
}
