// Temporary over-size rationale (B-29): parser coverage and legacy key handling are
// still co-located. Closure target is <=250 lines after extracting keyed-field
// handlers by domain while preserving parse compatibility.
use super::error::ImportError;
use super::model::{
    ProxmoxDiskEntry, ProxmoxHostPciEntry, ProxmoxNetEntry, ProxmoxUsbEntry, ProxmoxVmConfig,
};
use std::collections::HashMap;

pub fn parse_proxmox_config(input: &str) -> Result<ProxmoxVmConfig, ImportError> {
    let mut result = ProxmoxVmConfig::default();
    let mut in_non_root_section = false;

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if is_section_header(line) {
            in_non_root_section = true;
            continue;
        }

        if in_non_root_section {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(ImportError::ParseError(format!(
                "invalid line {}: '{}'",
                line_no + 1,
                raw_line
            )));
        };

        let key = key.trim().to_string();
        let value = value.trim().to_string();

        if let Some((bus, index)) = parse_bus_key(&key) {
            result
                .disks
                .push(parse_disk_entry(&key, &bus, index, &value));
            continue;
        }

        if let Some(index) = key
            .strip_prefix("net")
            .and_then(|x| x.parse::<usize>().ok())
        {
            result.networks.push(parse_net_entry(&key, index, &value));
            continue;
        }

        if let Some(index) = key
            .strip_prefix("hostpci")
            .and_then(|x| x.parse::<usize>().ok())
        {
            result
                .host_pci
                .push(parse_hostpci_entry(&key, index, &value));
            continue;
        }

        if let Some(index) = key
            .strip_prefix("usb")
            .and_then(|x| x.parse::<usize>().ok())
        {
            result.usb.push(parse_usb_entry(&key, index, &value));
            continue;
        }

        result.scalars.insert(key, value);
    }

    result.disks.sort_by_key(|entry| entry.index);
    result.networks.sort_by_key(|entry| entry.index);
    result.host_pci.sort_by_key(|entry| entry.index);
    result.usb.sort_by_key(|entry| entry.index);

    Ok(result)
}

fn parse_bus_key(key: &str) -> Option<(String, usize)> {
    for bus in ["scsi", "sata", "ide", "virtio"] {
        if let Some(index) = key.strip_prefix(bus).and_then(|x| x.parse::<usize>().ok()) {
            return Some((bus.to_string(), index));
        }
    }
    None
}

fn is_section_header(line: &str) -> bool {
    line.starts_with('[') && line.ends_with(']') && line.len() > 2
}

fn parse_disk_entry(key: &str, bus: &str, index: usize, value: &str) -> ProxmoxDiskEntry {
    let mut tokens = value.split(',');
    let source = tokens.next().unwrap_or("").trim().to_string();
    let options = parse_key_value_tokens(tokens);

    ProxmoxDiskEntry {
        key: key.to_string(),
        index,
        bus: bus.to_string(),
        source,
        options,
    }
}

fn parse_net_entry(key: &str, index: usize, value: &str) -> ProxmoxNetEntry {
    let mut model = "virtio".to_string();
    let mut mac: Option<String> = None;
    let mut options: HashMap<String, String> = HashMap::new();

    for (i, token) in value.split(',').enumerate() {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if let Some((k, v)) = token.split_once('=') {
            let k = k.trim();
            let v = v.trim();

            if i == 0 && !is_known_net_option(k) {
                model = k.to_string();
                mac = Some(v.to_string());
            } else {
                options.insert(k.to_string(), v.to_string());
            }
        }
    }

    ProxmoxNetEntry {
        key: key.to_string(),
        index,
        model,
        mac,
        options,
    }
}

fn is_known_net_option(key: &str) -> bool {
    matches!(
        key,
        "bridge" | "tag" | "trunks" | "firewall" | "queues" | "rate" | "link_down" | "mtu" | "name"
    )
}

fn parse_hostpci_entry(key: &str, index: usize, value: &str) -> ProxmoxHostPciEntry {
    let mut tokens = value.split(',');
    let host = tokens.next().unwrap_or("").trim().to_string();
    let options = parse_key_value_tokens(tokens);

    ProxmoxHostPciEntry {
        key: key.to_string(),
        index,
        host,
        options,
    }
}

fn parse_usb_entry(key: &str, index: usize, value: &str) -> ProxmoxUsbEntry {
    let mut tokens = value.split(',');
    let host = tokens.next().unwrap_or("").trim().to_string();
    let options = parse_key_value_tokens(tokens);

    ProxmoxUsbEntry {
        key: key.to_string(),
        index,
        host,
        options,
    }
}

fn parse_key_value_tokens<'a, I>(tokens: I) -> HashMap<String, String>
where
    I: Iterator<Item = &'a str>,
{
    let mut options = HashMap::new();

    for token in tokens {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if let Some((k, v)) = token.split_once('=') {
            options.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    options
}

#[cfg(test)]
mod tests {
    use super::parse_proxmox_config;

    #[test]
    fn parse_minimal_scalars_and_devices() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-100
            memory: 8192
            cores: 4
            sockets: 1
            scsi0: local-lvm:vm-100-disk-0,discard=on,size=64G
            net0: virtio=BC:24:11:FF:76:89,bridge=vmbr0,firewall=1
            hostpci0: 0000:03:10.4,pcie=1
            usb0: host=1-2
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(
            parsed.scalars.get("name").map(String::as_str),
            Some("vm-100")
        );
        assert_eq!(parsed.disks.len(), 1);
        assert_eq!(parsed.networks.len(), 1);
        assert_eq!(parsed.host_pci.len(), 1);
        assert_eq!(parsed.usb.len(), 1);
        assert_eq!(parsed.networks[0].model, "virtio");
        assert_eq!(parsed.networks[0].mac.as_deref(), Some("BC:24:11:FF:76:89"));
    }

    #[test]
    fn rejects_invalid_root_line_without_colon() {
        let err = parse_proxmox_config("invalid line").expect_err("must fail");
        let msg = err.to_string();
        assert!(msg.contains("invalid line"));
    }

    #[test]
    fn ignores_snapshot_sections() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-200
            memory: 4096
            net0: virtio=BC:24:11:AA:BB:CC,bridge=vmbr0

            [intermediate_20240524]
            snaptime: 1716500000
            memory: 2048
            [older_snapshot]
            memory: 1024
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(
            parsed.scalars.get("memory").map(String::as_str),
            Some("4096")
        );
        assert_eq!(parsed.networks.len(), 1);
    }

    #[test]
    fn parses_multiple_disk_buses_and_sorts_by_index() {
        let parsed = parse_proxmox_config(
            r#"
            scsi2: local-lvm:disk2,size=100G
            sata0: local:iso/debian.iso,media=cdrom
            ide1: none,media=cdrom
            virtio3: local-lvm:virtio3,discard=on
            scsi0: local-lvm:disk0,size=20G
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(parsed.disks.len(), 5);
        assert_eq!(parsed.disks[0].key, "sata0");
        assert_eq!(parsed.disks[1].key, "scsi0");
        assert_eq!(parsed.disks[2].key, "ide1");
        assert_eq!(parsed.disks[3].key, "scsi2");
        assert_eq!(parsed.disks[4].key, "virtio3");
    }

    #[test]
    fn parses_network_with_known_option_first_token() {
        let parsed =
            parse_proxmox_config("net0: bridge=vmbr0,tag=20").expect("parse should succeed");

        assert_eq!(parsed.networks.len(), 1);
        assert_eq!(parsed.networks[0].model, "virtio");
        assert_eq!(parsed.networks[0].mac, None);
        assert_eq!(
            parsed.networks[0].options.get("bridge").map(String::as_str),
            Some("vmbr0")
        );
    }

    #[test]
    fn parses_network_with_custom_model_and_mac() {
        let parsed = parse_proxmox_config("net1: e1000=52:54:00:12:34:56,bridge=vmbr1")
            .expect("parse should succeed");

        assert_eq!(parsed.networks.len(), 1);
        assert_eq!(parsed.networks[0].model, "e1000");
        assert_eq!(parsed.networks[0].mac.as_deref(), Some("52:54:00:12:34:56"));
        assert_eq!(parsed.networks[0].index, 1);
    }

    #[test]
    fn parses_hostpci_with_options() {
        let parsed = parse_proxmox_config("hostpci0: 0000:03:00.0,pcie=1,x-vga=1")
            .expect("parse should succeed");

        assert_eq!(parsed.host_pci.len(), 1);
        assert_eq!(parsed.host_pci[0].host, "0000:03:00.0");
        assert_eq!(
            parsed.host_pci[0].options.get("pcie").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn parses_usb_entries_sorted_by_index() {
        let parsed = parse_proxmox_config(
            r#"
            usb2: host=1-4
            usb0: host=0451:16a0
            usb1: host=1-2
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(parsed.usb.len(), 3);
        assert_eq!(parsed.usb[0].key, "usb0");
        assert_eq!(parsed.usb[1].key, "usb1");
        assert_eq!(parsed.usb[2].key, "usb2");
    }

    #[test]
    fn skips_comments_and_empty_lines() {
        let parsed = parse_proxmox_config(
            r#"
            # comment

            name: vm-commented

            # another
            memory: 2048
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(
            parsed.scalars.get("name").map(String::as_str),
            Some("vm-commented")
        );
        assert_eq!(
            parsed.scalars.get("memory").map(String::as_str),
            Some("2048")
        );
    }

    #[test]
    fn overwrites_scalar_when_key_repeated() {
        let parsed = parse_proxmox_config(
            r#"
            memory: 2048
            memory: 4096
            "#,
        )
        .expect("parse should succeed");

        assert_eq!(
            parsed.scalars.get("memory").map(String::as_str),
            Some("4096")
        );
    }
}
