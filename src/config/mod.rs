//! Configuration module for ezkvm
//!
//! This module handles parsing and validation of YAML configuration files
//! that define virtual machine specifications.

mod central;
pub mod devices;
mod entrypoint;
mod loader;
mod platform;
pub mod system;
pub mod validation;
mod vm_options;
mod vm_schema;

#[allow(unused_imports)]
pub use central::{CentralConfig, LocationsConfig, LookingGlassOptions, ToolsConfig};
#[allow(unused_imports)]
pub use platform::{
    AudioDeviceConfig, BallooningConfig, GuestAgentConfig, HostPciConfig, HypervConfig,
    InputDeviceConfig, IscsiDiskConfig, IvshmemConfig, NumaConfig, QmpConfig, QmpSocketType,
    SataControllerConfig, ScsiControllerConfig, SmbiosConfig, SpiceConfig, TpmConfig,
    UsbDeviceConfig, XhciControllerConfig,
};
#[allow(unused_imports)]
pub use vm_options::{RtcConfig, VmOptions};
#[allow(unused_imports)]
pub use vm_schema::{
    BootConfig, ControllersConfig, CpuConfig, DeviceConfig, DisplayConfig, DriveConfig, HostConfig,
    MemoryConfig, NetworkBackendConfig, NetworkConfig, SerialConfig, SystemConfig, VmConfig,
};

pub(crate) use entrypoint::{DEFAULT_CENTRAL_CONFIG_PATHS, DEFAULT_PROFILE_DIR};

#[cfg(test)]
mod tests;
