use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ProxmoxVmConfig {
    pub scalars: BTreeMap<String, String>,
    pub disks: Vec<ProxmoxDiskEntry>,
    pub networks: Vec<ProxmoxNetEntry>,
    pub host_pci: Vec<ProxmoxHostPciEntry>,
    pub usb: Vec<ProxmoxUsbEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxDiskEntry {
    pub key: String,
    pub index: usize,
    pub bus: String,
    pub source: String,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxNetEntry {
    pub key: String,
    pub index: usize,
    pub model: String,
    pub mac: Option<String>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxHostPciEntry {
    pub key: String,
    pub index: usize,
    pub host: String,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProxmoxUsbEntry {
    pub key: String,
    pub index: usize,
    pub host: String,
    pub options: HashMap<String, String>,
}
