use derive_getters::Getters;
use derive_new::new;
use std::fmt::Debug;

#[allow(dead_code)]
pub trait PcieDevice: Debug + Sync + Send + 'static {}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Getters, new)]
pub struct PcieAddress {
    device: u8,
    function: u8,
}
