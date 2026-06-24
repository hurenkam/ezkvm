use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Gpu {
    Standard,
    Qxl,
    Virtio,
    Headless,
    Passthrough,
}

pub trait GpuApi: fmt::Display {
    fn gpu_type(&self) -> &Gpu;
    fn qemu_args(&self) -> Vec<String>;
}

pub struct GpuModelBuilder {}

impl GpuModelBuilder {
    pub fn build(gpu: Gpu) -> Arc<dyn GpuApi> {
        Arc::new(GpuModel { gpu })
    }
}

struct GpuModel {
    gpu: Gpu,
}

impl GpuApi for GpuModel {
    fn gpu_type(&self) -> &Gpu {
        &self.gpu
    }

    fn qemu_args(&self) -> Vec<String> {
        match &self.gpu {
            Gpu::Standard => vec!["-vga".to_string(), "std".to_string()],
            Gpu::Qxl => vec!["-vga".to_string(), "qxl".to_string()],
            Gpu::Virtio => vec!["-device".to_string(), "virtio-vga".to_string()],
            Gpu::Headless => vec!["-vga".to_string(), "none".to_string()],
            Gpu::Passthrough => vec![],
        }
    }
}

impl fmt::Display for GpuModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.gpu {
            Gpu::Standard => write!(f, "Standard VGA"),
            Gpu::Qxl => write!(f, "QXL"),
            Gpu::Virtio => write!(f, "Virtio GPU"),
            Gpu::Headless => write!(f, "Headless GPU"),
            Gpu::Passthrough => write!(f, "Passthrough GPU"),
        }
    }
}
