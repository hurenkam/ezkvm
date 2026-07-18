use derive_getters::Getters;
use derive_new::new;

use crate::runtime::StorageDevice;
use std::fmt::Debug;

#[allow(dead_code)]
pub trait IdeDevice: StorageDevice + Debug + Sync + Send + 'static {}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Getters, new)]
pub struct IdeAddress {
    channel: u8,
    device: u8,
}
