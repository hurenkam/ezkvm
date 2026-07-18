use std::fmt::Debug;

use derive_getters::Getters;
use derive_new::new;

#[allow(dead_code)]
pub trait PciDevice: Debug + Sync + Send + 'static {}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Getters, new)]
pub struct PciAddress {
    device: u8,
    function: u8,
}
