use std::fmt::Debug;
use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::network::NetworkDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct NetworkBridgeDevice {
    bridge: String,
    mac: String,
    driver: String
}

#[typetag::deserialize(name = "bridge")]
impl NetworkDevice for NetworkBridgeDevice { }

impl QemuDevice for NetworkBridgeDevice {
    fn get_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-netdev id=hostnet{},type=bridge,br=vmbr0",index),
            format!("-device id=net{},driver={},netdev=hostnet{},mac={},bus=pci.1,addr=0x0",index,self.driver,index,self.mac),
        ]
    }
}
