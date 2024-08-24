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

pub trait SwtpmArgs {
    fn get_swtpm_args(&self, index: usize) -> Vec<String>;
}

pub trait QemuArgs {
    fn get_qemu_args(&self, index: usize) -> Vec<String>;
}

pub trait LgClientArgs {
    fn get_lg_client_args(&self, index: usize) -> Vec<String>;
}
