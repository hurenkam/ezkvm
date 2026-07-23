// Phase 3 Plan 03-02: ProxmoxVmConf and sub-structs

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxDiskConf {
    pub volume: String,
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxNetConf {
    pub model: String,
    pub mac: String,
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxHostPciConf {
    pub bdf: String,
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxUsbConf {
    pub host: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxAudioConf {
    pub device: Option<String>,
    pub driver: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxTpmConf {
    pub volume: String,
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxEfiDiskConf {
    pub volume: String,
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxSerialConf {
    pub socket: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProxmoxVmConf {
    pub scsi:    BTreeMap<u8, ProxmoxDiskConf>,
    pub sata:    BTreeMap<u8, ProxmoxDiskConf>,
    pub ide:     BTreeMap<u8, ProxmoxDiskConf>,
    pub virtio:  BTreeMap<u8, ProxmoxDiskConf>,
    pub net:     BTreeMap<u8, ProxmoxNetConf>,
    pub hostpci: BTreeMap<u8, ProxmoxHostPciConf>,
    pub usb:     BTreeMap<u8, ProxmoxUsbConf>,
    pub serial:  BTreeMap<u8, ProxmoxSerialConf>,

    pub efidisk:  Option<ProxmoxEfiDiskConf>,
    pub tpmstate: Option<ProxmoxTpmConf>,
    pub audio:    Option<ProxmoxAudioConf>,

    pub memory:  Option<u64>,
    pub machine: Option<String>,
    pub bios:    Option<String>,
    pub cpu:     Option<String>,
    pub args:    Option<String>,
    pub vga:     Option<String>,
    pub cores:   Option<u8>,
    pub sockets: Option<u8>,
    pub name:    Option<String>,
    pub ostype:  Option<String>,
    pub numa:    Option<bool>,
    pub boot:    Option<String>,
    pub scsihw:  Option<String>,
    pub parent:  Option<String>,
    pub meta:    Option<String>,
    pub smbios1: Option<String>,
    pub tablet:  Option<bool>,
    pub vmgenid: Option<String>,
    pub agent:   Option<String>,
}

// FromStr implemented in Task 03-02-B after parse_sub_options() is available (Plan 03-03)

use std::str::FromStr;

use crate::config::proxmox::error::ProxmoxParseError;
use crate::config::proxmox::parser::{
    parse_disk_raw, parse_efidisk_raw, parse_hostpci_raw, parse_net_raw, parse_serial_raw,
    parse_tpmstate_raw, parse_usb_raw, parse_audio_raw, split_sections,
};

impl FromStr for ProxmoxVmConf {
    type Err = ProxmoxParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (entries, _snapshots) = split_sections(s);
        let mut conf = ProxmoxVmConf::default();

        for (key, value) in entries {
            // scsihw must be checked BEFORE any starts_with("scsi") prefix
            if key == "scsihw" {
                conf.scsihw = Some(value);
            } else if key == "memory" {
                conf.memory = value.parse::<u64>().ok();
            } else if key == "cores" {
                conf.cores = value.parse::<u8>().ok();
            } else if key == "sockets" {
                conf.sockets = value.parse::<u8>().ok();
            } else if key == "numa" {
                conf.numa = Some(value == "1");
            } else if key == "tablet" {
                conf.tablet = Some(value == "1");
            } else if key == "cpu" {
                conf.cpu = Some(value);
            } else if key == "machine" {
                conf.machine = Some(value);
            } else if key == "bios" {
                conf.bios = Some(value);
            } else if key == "vga" {
                conf.vga = Some(value);
            } else if key == "args" {
                conf.args = Some(value);
            } else if key == "name" {
                conf.name = Some(value);
            } else if key == "ostype" {
                conf.ostype = Some(value);
            } else if key == "boot" {
                conf.boot = Some(value);
            } else if key == "parent" {
                conf.parent = Some(value);
            } else if key == "meta" {
                conf.meta = Some(value);
            } else if key == "smbios1" {
                conf.smbios1 = Some(value);
            } else if key == "vmgenid" {
                conf.vmgenid = Some(value);
            } else if key == "agent" {
                conf.agent = Some(value);
            } else if key == "audio0" {
                conf.audio = parse_audio_raw(&value).ok();
            } else if key == "efidisk0" {
                conf.efidisk = parse_efidisk_raw(&value).ok();
            } else if let Some(rest) = key.strip_prefix("tpmstate") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if idx == 0 {
                        conf.tpmstate = parse_tpmstate_raw(&value).ok();
                    }
                }
            } else if let Some(rest) = key.strip_prefix("scsi") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(disk) = parse_disk_raw(&value) {
                        conf.scsi.insert(idx, disk);
                    }
                }
            } else if let Some(rest) = key.strip_prefix("sata") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(disk) = parse_disk_raw(&value) {
                        conf.sata.insert(idx, disk);
                    }
                }
            } else if let Some(rest) = key.strip_prefix("ide") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(disk) = parse_disk_raw(&value) {
                        conf.ide.insert(idx, disk);
                    }
                }
            } else if let Some(rest) = key.strip_prefix("virtio") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(disk) = parse_disk_raw(&value) {
                        conf.virtio.insert(idx, disk);
                    }
                }
            } else if let Some(rest) = key.strip_prefix("net") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(net) = parse_net_raw(&value) {
                        conf.net.insert(idx, net);
                    }
                }
            } else if let Some(rest) = key.strip_prefix("hostpci") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(pci) = parse_hostpci_raw(&value) {
                        conf.hostpci.insert(idx, pci);
                    }
                }
            } else if let Some(rest) = key.strip_prefix("usb") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(usb) = parse_usb_raw(&value) {
                        conf.usb.insert(idx, usb);
                    }
                }
            } else if let Some(rest) = key.strip_prefix("serial") {
                if let Ok(idx) = rest.parse::<u8>() {
                    if let Ok(serial) = parse_serial_raw(&value) {
                        conf.serial.insert(idx, serial);
                    }
                }
            }
            // unknown keys are silently ignored
        }

        Ok(conf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxmox_vm_conf_from_108_conf() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = std::path::Path::new(&manifest).join("input/felucia/108.conf");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));

        let conf = ProxmoxVmConf::from_str(&content).unwrap();

        assert_eq!(conf.cpu.as_deref(), Some("host"), "cpu should be 'host'");
        assert!(conf.efidisk.is_some(), "efidisk0 should be parsed");
        assert!(conf.tpmstate.is_some(), "tpmstate0 should be parsed");
        assert!(conf.hostpci.contains_key(&0), "hostpci0 should be parsed");
        assert!(conf.net.contains_key(&0), "net0 should be parsed");
        assert_eq!(conf.net[&0].mac, "BC:24:11:3A:21:B7", "MAC preserved");
        assert!(conf.scsi.contains_key(&0), "scsi0 should be parsed");
        assert!(conf.scsi.contains_key(&1), "scsi1 should be parsed");
        assert!(conf.audio.is_some(), "audio0 should be parsed");
        assert!(conf.usb.contains_key(&0), "usb0 should be parsed");
        assert!(conf.args.is_some(), "args should be preserved verbatim");
        assert_eq!(conf.scsihw.as_deref(), Some("pvscsi"), "scsihw should be 'pvscsi'");
    }
}
