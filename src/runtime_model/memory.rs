use derive_new::new;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct Memory {
    size: usize,
}
impl Memory {
    pub fn kilobytes(kb: usize) -> Self {
        Self { size: kb * 1024 }
    }
    pub fn megabytes(mb: usize) -> Self {
        Self {
            size: mb * 1024 * 1024,
        }
    }
    pub fn gigabytes(gb: usize) -> Self {
        Self {
            size: gb * 1024 * 1024 * 1024,
        }
    }
    pub fn terabytes(tb: usize) -> Self {
        Self {
            size: tb * 1024 * 1024 * 1024 * 1024,
        }
    }
}
