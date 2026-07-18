use derive_getters::Getters;
use derive_new::new;
use std::fmt::Debug;

use crate::runtime::StorageDevice;

#[allow(dead_code)]
pub trait SataDevice: StorageDevice + Debug + Sync + Send + 'static {}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Getters, new)]
pub struct SataAddress {
    port: u8,
    device: u8,
}
