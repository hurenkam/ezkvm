use std::fmt::Display;

use derive_getters::Getters;
use derive_new::new;

use crate::runtime_model::StorageResource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageDeviceKind {
    Hdd,
    Ssd,
    Cdrom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageCachePolicy {
    None,
    Writeback,
    Writethrough,
    Unsafe,
    Directsync,
}

impl StorageCachePolicy {
    pub fn as_qemu_cache_value(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Writeback => "writeback",
            Self::Writethrough => "writethrough",
            Self::Unsafe => "unsafe",
            Self::Directsync => "directsync",
        }
    }

    pub fn as_proxmox_value(&self) -> &'static str {
        self.as_qemu_cache_value()
    }

    pub fn from_proxmox_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "writeback" => Some(Self::Writeback),
            "writethrough" => Some(Self::Writethrough),
            "unsafe" => Some(Self::Unsafe),
            "directsync" => Some(Self::Directsync),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, Getters, new)]
pub struct StorageDeviceOptions {
    #[new(default)]
    rotation_rate: Option<u16>,
    #[new(default)]
    boot_index: Option<u16>,
    #[new(default)]
    device_id: Option<String>,
    #[new(default)]
    cache_policy: Option<StorageCachePolicy>,
}

impl StorageDeviceOptions {
    pub fn from_parts(
        rotation_rate: Option<u16>,
        boot_index: Option<u16>,
        device_id: Option<String>,
        cache_policy: Option<StorageCachePolicy>,
    ) -> Self {
        Self {
            rotation_rate,
            boot_index,
            device_id,
            cache_policy,
        }
    }
}

#[derive(Debug, Clone, Getters, new)]
pub struct Cdrom {
    resource: StorageResource,
    #[new(default)]
    options: StorageDeviceOptions,
}
impl Cdrom {
    pub fn with_options(resource: StorageResource, options: StorageDeviceOptions) -> Self {
        Self { resource, options }
    }
}
impl Display for Cdrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cdrom: {:?}", self.resource)
    }
}

#[derive(Debug, Clone, Getters, new)]
pub struct Hdd {
    resource: StorageResource,
    #[new(default)]
    options: StorageDeviceOptions,
}
impl Hdd {
    pub fn with_options(resource: StorageResource, options: StorageDeviceOptions) -> Self {
        Self { resource, options }
    }
}
impl Display for Hdd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hdd: {:?}", self.resource)
    }
}

#[derive(Debug, Clone, Getters, new)]
pub struct Ssd {
    resource: StorageResource,
    #[new(default)]
    options: StorageDeviceOptions,
}
impl Ssd {
    pub fn with_options(resource: StorageResource, options: StorageDeviceOptions) -> Self {
        Self { resource, options }
    }
}
impl Display for Ssd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ssd: {:?}", self.resource)
    }
}
