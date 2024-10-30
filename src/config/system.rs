use crate::config::qemu_device::QemuDevice;
use crate::config::system::bios::Bios;
use crate::config::system::chipset::Chipset;
use crate::config::system::cpu::Cpu;
use crate::config::system::memory::Memory;
use crate::config::system::tpm::Tpm;
use crate::config::{default_when_missing, Config};
use derive_getters::Getters;
use serde::Deserialize;
use typetag::serde;

mod bios;
mod chipset;
mod cpu;
mod memory;
mod tpm;

#[allow(dead_code)]
#[derive(Deserialize, Default, Debug, Getters)]
pub struct System {
    #[serde(default, deserialize_with = "default_when_missing")]
    chipset: Box<dyn Chipset>,
    #[serde(default, deserialize_with = "default_when_missing")]
    bios: Box<dyn Bios>,
    #[serde(default, deserialize_with = "default_when_missing")]
    memory: Memory,
    #[serde(default, deserialize_with = "default_when_missing")]
    cpu: Cpu,
    #[serde(default, deserialize_with = "default_when_missing")]
    tpm: Box<dyn Tpm>,
}

impl QemuDevice for System {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.chipset.get_qemu_args(0));
        result.extend(self.bios.get_qemu_args(0));
        result.extend(self.memory.get_qemu_args(0));
        result.extend(self.cpu.get_qemu_args(0));
        result.extend(self.tpm.get_qemu_args(0));
        result
    }

    fn pre_start(&self, config: &Config) {
        self.tpm.pre_start(config);
    }
}
