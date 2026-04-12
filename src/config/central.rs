use serde::{Deserialize, Serialize};

/// Central tool configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CentralConfig {
    /// Tool paths
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Directory locations
    #[serde(default)]
    pub locations: LocationsConfig,

    /// Looking Glass client options
    #[serde(default)]
    pub looking_glass: LookingGlassOptions,
}

/// Tool paths configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    /// Path to swtpm executable
    pub swtpm: Option<String>,

    /// Path to remote-viewer executable
    pub remote_viewer: Option<String>,

    /// Path to looking-glass-client executable
    pub looking_glass: Option<String>,
}

/// Directory locations configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocationsConfig {
    /// Runtime directory for PID files, sockets, etc.
    pub run_dir: Option<String>,

    /// Directory containing OVMF firmware files
    pub ovmf_dir: Option<String>,

    /// Default directory for VM configuration files
    pub vm_dir: Option<String>,

    /// Directory for VM templates
    pub template_dir: Option<String>,

    /// Directory containing reusable VM profile files
    pub profile_dir: Option<String>,
}

/// Looking Glass client options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LookingGlassOptions {
    /// Launch the client in fullscreen mode.
    pub full_screen: Option<bool>,

    /// Initial window size in WIDTHxHEIGHT format.
    pub size: Option<String>,

    /// Grab the keyboard when focused.
    pub grab_keyboard: Option<bool>,

    /// Escape key name used to release keyboard grab.
    pub escape_key: Option<String>,
}
