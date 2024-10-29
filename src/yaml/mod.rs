pub mod config;
mod display;
pub mod general;
mod gpu;
pub mod host;
pub mod looking_glass;
pub mod network;
pub mod spice;
pub mod storage;
pub mod system;
/*
pub trait SwtpmArgs {
    fn get_swtpm_args(&self, index: usize) -> Vec<String>;
}

pub trait QemuDevice {
    fn get_qemu_args(&self, index: usize) -> Vec<String>;
}
*/
pub trait LgClientArgs {
    fn get_lg_client_args(&self, index: usize) -> Vec<String>;
}
