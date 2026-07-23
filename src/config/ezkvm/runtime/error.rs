use thiserror::Error;

/// Typed errors for the Runtime ↔ ezkvm YAML conversion boundary (both
/// `TryFrom<Runtime> for ConfigSchema` and `TryFrom<ConfigSchema> for Runtime`).
///
/// Per D-01 (Phase 6 context), no bare `String` error is permitted at this boundary —
/// every failure mode must be a named, actionable variant.
#[derive(Debug, Error)]
pub enum YamlRuntimeError {
    #[error("runtime is missing required Memory root device")]
    MissingMemory,

    #[error("runtime is missing required Chipset root device")]
    MissingChipset,

    #[error("unsupported chipset '{chipset}' for YAML conversion")]
    UnsupportedChipset { chipset: String },

    #[error("unsupported TPM type '{tpm_type}'; only emulated swtpm is supported")]
    UnsupportedTpmType { tpm_type: String },

    #[error("unsupported PCIe device '{device_type}' for YAML conversion")]
    UnsupportedPcieDevice { device_type: String },

    #[error("HostPci passthrough must be on the PCIe bus, not the PCI bus")]
    UnsupportedPciPassthroughOnPciBus,

    #[error("resource '{id}' referenced by device schema not found in host resources")]
    ResourceNotFound { id: String },

    #[error("unsupported USB bus {bus}; only bus 0 is modelled")]
    UnsupportedUsbBus { bus: u8 },

    #[error("unsupported SATA bus {bus}; only bus 0 is modelled")]
    UnsupportedSataBus { bus: u8 },

    #[error("duplicate PCIe address {device}:{function} during runtime assembly")]
    DuplicatePcieAddress { device: u8, function: u8 },

    #[error("i440fx chipset with attached devices is not supported in YAML conversion")]
    I440fxWithDevices,
}
