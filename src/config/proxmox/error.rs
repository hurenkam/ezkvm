#[derive(Debug, thiserror::Error)]
pub enum ProxmoxParseError {
    #[error("invalid key-value line: {line}")]
    InvalidLine { line: String },

    #[error("malformed snapshot header: {header}")]
    InvalidSection { header: String },

    #[error("invalid sub-option in field '{field}': {raw}")]
    InvalidSubOption { field: String, raw: String },

    #[error("required field missing: {field}")]
    MissingRequiredField { field: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ProxmoxImportError {
    #[error("vm_conf has no memory field")]
    MissingMemory,

    #[error("machine type '{machine}' is not supported (only q35 variants)")]
    UnsupportedMachine { machine: String },

    #[error("volume reference '{raw}' has no ':' separator — cannot resolve storage pool")]
    InvalidVolumeRef { raw: String },

    #[error("storage pool '{pool}' not found in storage.cfg")]
    UnknownStorage { pool: String },

    #[error("storage pool '{pool}' has unsupported type '{storage_type}'")]
    UnsupportedStorageType { pool: String, storage_type: String },

    #[error("storage pool '{pool}' is missing required property '{property}'")]
    MissingStorageProperty { pool: String, property: String },

    #[error("malformed USB host identity '{raw}' (expected '<bus>-<port>' or '<vendor>:<product>')")]
    MalformedUsbHostIdentity { raw: String },
}
