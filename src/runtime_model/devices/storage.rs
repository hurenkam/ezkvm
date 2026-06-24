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

#[derive(Debug, Clone, Getters, new)]
pub struct Cdrom {
    resource: StorageResource,
}
impl Display for Cdrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cdrom: {:?}", self.resource)
    }
}

#[derive(Debug, Clone, Getters, new)]
pub struct Hdd {
    resource: StorageResource,
}
impl Display for Hdd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hdd: {:?}", self.resource)
    }
}

#[derive(Debug, Clone, Getters, new)]
pub struct Ssd {
    resource: StorageResource,
}
impl Display for Ssd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ssd: {:?}", self.resource)
    }
}
