use serde::Deserialize;
use crate::config::config::{QemuDevice};
use crate::config::system::System;

const PVE_CONFIG_FILE: &str = "/mnt/usr/share/qemu-server/pve-q35-4.0.cfg";

#[derive(Deserialize,Debug,Clone)]
pub struct SystemQ35 {}
impl SystemQ35 {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self{})
    }
}
impl QemuDevice for SystemQ35 {
    fn get_args(&self, index: usize) -> Vec<String> {
        vec![
            "-machine hpet=off,type=pc-q35-8.1".to_string(),
            "-rtc driftfix=slew,base=localtime".to_string(),
            "-global kvm-pit.lost_tick_policy=discard".to_string(),
            format!("-readconfig {}",PVE_CONFIG_FILE),
            "-device qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b".to_string(),
            "-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b".to_string(),
            "-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5".to_string(),
        ]
    }
}

#[typetag::deserialize(name = "q35")]
impl System for SystemQ35 {}
