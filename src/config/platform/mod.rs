mod hyperv;
mod monitoring;
mod passthrough;
mod peripherals;
mod storage;
mod tpm_guest;

pub use hyperv::HypervConfig;
pub use monitoring::{NumaConfig, QmpConfig, QmpSocketType, SmbiosConfig};
pub use passthrough::{HostPciConfig, UsbDeviceConfig, XhciControllerConfig};
pub use peripherals::{AudioDeviceConfig, InputDeviceConfig, IvshmemConfig, SpiceConfig};
pub use storage::{IscsiDiskConfig, SataControllerConfig, ScsiControllerConfig};
pub use tpm_guest::{BallooningConfig, GuestAgentConfig, TpmConfig};
