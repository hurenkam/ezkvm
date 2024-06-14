use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::gpu::Gpu;

#[derive(Deserialize,Debug,Clone)]
pub struct QxlVga {

}

impl QemuDevice for QxlVga {
    fn get_args(&self, index: usize) -> Vec<String> {
        let offset = 2;
        vec![
            format!("-device qxl-vga,id=vga,bus=pcie.0,addr={:#x}", offset + index),
        ]
    }
}

#[typetag::deserialize(name = "qxl-vga")]
impl Gpu for QxlVga {}
