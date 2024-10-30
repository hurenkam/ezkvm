mod qemu_device;
mod system;

pub use qemu_device::QemuDevice;
use serde::{Deserialize, Deserializer};
pub use system::System;

pub fn default_when_missing<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let option = Option::deserialize(deserializer)?;
    Ok(option.unwrap_or_default())
}
