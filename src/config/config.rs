use std::fmt::Debug;
use serde::{Deserialize, Deserializer};
use log::info;
use typetag::serde;
use crate::config::bios::Bios;
use crate::config::cpu::Cpu;
use crate::config::display::Display;
use crate::config::gpu::Gpu;
use crate::config::memory::Memory;
use crate::config::network::NetworkDevice;
use crate::config::storage::StorageDevice;
use crate::config::system::System;

pub trait QemuDevice: Debug {
    fn get_args(&self, _index: usize) -> Vec<String>;
}

#[derive(Deserialize,Debug)]
pub struct Config {
    name: String,
    #[serde(default, deserialize_with = "default_when_missing")]
    system: Box<dyn System>,
    #[serde(default, deserialize_with = "default_when_missing")]
    bios: Box<dyn Bios>,
    #[serde(default, deserialize_with = "default_when_missing")]
    cpu: Cpu,
    #[serde(default, deserialize_with = "default_when_missing")]
    memory: Memory,
    display: Vec<Box<dyn Display>>,
    gpu: Vec<Box<dyn Gpu>>,
    network: Vec<Box<dyn NetworkDevice>>,
    storage: Vec<Box<dyn StorageDevice>>
}

impl QemuDevice for Config {
    fn get_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![
            "-accel kvm".to_string(),
            //"-daemonize".to_string(),
            "-nodefaults".to_string(),
            format!("-monitor unix:/var/ezkvm/{}.monitor,server,nowait",self.name),
            format!("-chardev socket,id=qmp,path=/var/ezkvm/{}.qmp,server=on,wait=off",self.name),
            "-mon chardev=qmp,mode=control".to_string(),
            "-chardev socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5".to_string(),
            "-mon chardev=qmp-event,mode=control".to_string(),
        ];

        result.extend(self.system.get_args(0));
        result.extend(self.bios.get_args(0));
        result.extend(self.cpu.get_args(0));
        result.extend(self.memory.get_args(0));

        for (i,display) in self.display.iter().enumerate() {
            result.extend(display.get_args(i));
        }

        for (i,gpu) in self.gpu.iter().enumerate() {
            result.extend(gpu.get_args(i));
        }

        for (i,network) in self.network.iter().enumerate() {
            result.extend(network.get_args(i));
        }

        for (i,disk) in self.storage.iter().enumerate() {
            result.extend(disk.get_args(i));
        }

        let mut args = "qemu-system-x86_64".to_string();
        for arg in result {
            info!("{}",arg);
            args = format!("{} {}",args,arg).to_string();
        }

        args.split_whitespace().map(str::to_string).collect()
    }
}

fn default_when_missing<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de> + Default,
{
    let option = Option::deserialize(deserializer)?;
    Ok(option.unwrap_or_default())
}
