//! Configuration module - parsing, validation, and merging of VM specs.
//!
//! Handles complete lifecycle of VM configurations:
//! - YAML parsing with serde and schema validation
//! - Profile-aware merging with precedence policies
//! - Device ID assignment and normalization
//! - Proxmox VM import mapping
//! - Central tool configuration and override threading
//!
//! # Architecture
//! - `vm_schema/`: Core VmConfig type hierarchy (system, devices, controllers, host)
//! - `platform/`: Platform-specific config types (TPM, guest-agent, SPICE, etc.)
//! - `loader/`: Profile loading, merge policies, and YAML composition
//! - `validation/`: Multi-stage config validation with domain checks
//! - `central.rs`: Central tool configuration and CLI override struct
//!
//! # Design
//! - Compact output: omits fields at semantic defaults via skip_serializing_if
//! - Backward compatible: deserializer handles missing optional fields gracefully
//! - Fail-fast: validates after parsing, rejects invalid configs early
//! - Profile-aware: supports layered configurations with merge policies

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
pub use central::{
    CentralConfig, FirmwareHostCapabilities, HostCapabilitiesConfig, IntegrationHostCapabilities,
    LocationsConfig, LookingGlassCapability, LookingGlassOptions, NetworkHostCapabilities,
    ProgramCapability, RuntimeCliOverrides, RuntimeHostCapabilities, ToolsConfig,
    TpmHostCapabilities,
};
#[allow(unused_imports)]
pub use platform::{
    AppleSmcConfig, AudioDeviceConfig, BallooningConfig, GuestAgentConfig, HostPciConfig,
    HugepagesConfig, HypervConfig, InputDeviceConfig, IommuConfig, IscsiDiskConfig, IvshmemConfig,
    NumaConfig, QmpConfig, QmpSocketType, SataControllerConfig, ScsiControllerConfig, SmbiosConfig,
    SpiceConfig, TpmConfig, UsbDeviceConfig, VncConfig, XhciControllerConfig,
};
#[allow(unused_imports)]
pub use vm_options::{RtcConfig, VmOptions};
#[allow(unused_imports)]
pub use vm_schema::{
    BootConfig, ControllersConfig, CpuConfig, DeviceConfig, DisplayConfig, DriveConfig, HostConfig,
    MachineLayoutBus, MachineLayoutConfig, MachineLayoutDevice, MachineLayoutNode,
    MachineLayoutNodeKind, MemoryConfig, NetworkBackendConfig, NetworkConfig, SerialConfig,
    SystemConfig, VmConfig,
};

pub(crate) use entrypoint::{DEFAULT_CENTRAL_CONFIG_PATHS, DEFAULT_PROFILE_DIR};

#[cfg(test)]
mod tests;
