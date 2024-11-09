use crate::config::network::NetworkItem;
use crate::config::Config;
use std::fmt::Debug;

#[typetag::deserialize(tag = "type")]
pub trait NetworkPayload: Debug {
    fn pre_start(&self, _parent: &NetworkItem, _config: &Config) {}
    fn post_start(&self, _parent: &NetworkItem, _config: &Config) {}
    fn pre_stop(&self, _parent: &NetworkItem, _config: &Config) {}
    fn post_stop(&self, _parent: &NetworkItem, _config: &Config) {}
    fn pre_hibernate(&self, _parent: &NetworkItem, _config: &Config) {}
    fn post_hibernate(&self, _parent: &NetworkItem, _config: &Config) {}
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
    fn get_device_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}
