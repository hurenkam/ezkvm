use crate::config::Config;
use std::fmt::Debug;

pub trait QemuDevice: Debug {
    fn get_qemu_args(&self, index: usize) -> Vec<String>;
    fn pre_start(&self, _config: &Config) {}
    fn post_start(&self, _config: &Config) {}
    fn pre_stop(&self, _config: &Config) {}
    fn post_stop(&self, _config: &Config) {}
    fn pre_hibernate(&self, _config: &Config) {}
    fn post_hibernate(&self, _config: &Config) {}
}
