use crate::yaml::config::Config;
use std::fmt::Debug;

pub trait QemuDevice: Debug {
    fn get_qemu_args(&self, index: usize) -> Vec<String>;
    fn start(&self, config: &Config) {}
    fn stop(&self, config: &Config) {}
}
