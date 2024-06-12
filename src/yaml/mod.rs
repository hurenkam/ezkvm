pub mod config;
pub mod host;
pub mod system;
pub mod general;
pub mod spice;
pub mod storage;
pub mod network;
pub mod looking_glass;
mod gpu;
mod display;

pub trait QemuArgs {
    fn get_qemu_args(&self, index: usize) -> Vec<String>;
}
