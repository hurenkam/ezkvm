use derive_getters::Getters;
use derive_new::new;
use std::fmt::Debug;

use crate::runtime::StorageDevice;

#[allow(dead_code)]
pub trait ScsiDevice: StorageDevice + Debug + Send + Sync + 'static {}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Getters, new)]
pub struct ScsiAddress {
    target: u8,
    lun: u8,
}
