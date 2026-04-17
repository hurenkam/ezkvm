use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use super::super::{
    CentralConfig, DEFAULT_PROFILE_DIR,
    loader::{env, merge, policies},
    validation,
};

use super::super::{
    AppleSmcConfig, AudioDeviceConfig, BallooningConfig, GuestAgentConfig, HostPciConfig,
    HypervConfig, InputDeviceConfig, IommuConfig, IscsiDiskConfig, IvshmemConfig, QmpConfig,
    SataControllerConfig, ScsiControllerConfig, SmbiosConfig, SpiceConfig, TpmConfig,
    UsbDeviceConfig, VmOptions, XhciControllerConfig,
};
use super::{BootConfig, DeviceConfig, SystemConfig};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControllersConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scsi: Vec<ScsiControllerConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sata: Vec<SataControllerConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub xhci: Vec<XhciControllerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HostConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pci: Vec<HostPciConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usb: Vec<UsbDeviceConfig>,
}

/// Main configuration structure for a virtual machine.
/// Intentionally kept as one aggregate root to preserve a stable YAML contract and
/// explicit ownership of all top-level VM sections in one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Name of the virtual machine
    pub name: String,

    /// Backend to use (currently only "qemu" is supported)
    pub backend: String,

    /// Optional list of profile names to layer before applying VM overrides.
    /// Profile files are resolved from the configured profile directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<String>,

    /// System configuration (CPU, memory, etc.)
    pub system: SystemConfig,

    /// Device configuration
    #[serde(default)]
    pub devices: DeviceConfig,

    /// Canonical controller configuration.
    #[serde(default)]
    pub controllers: ControllersConfig,

    /// Canonical host passthrough configuration.
    #[serde(default)]
    pub host: HostConfig,

    /// SPICE display configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spice: Option<SpiceConfig>,

    /// iSCSI storage configuration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub iscsi_disks: Vec<IscsiDiskConfig>,

    /// Hyper-V enlightenments configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperv: Option<HypervConfig>,

    /// IOMMU / vIOMMU device configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iommu: Option<IommuConfig>,

    /// Additional options
    #[serde(default)]
    pub options: VmOptions,
}

impl VmConfig {
    /// Load configuration from a YAML file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let vm_value = Self::load_vm_value_from_file(path)?;
        let profile_names = Self::extract_profile_names(&vm_value)?;
        let profile_dir = Self::resolve_profile_dir()?;
        let merged_value = Self::build_merged_vm_value(vm_value, &profile_dir, &profile_names)?;
        Self::deserialize_and_validate(merged_value)
    }

    pub fn system_boot(&self) -> &BootConfig {
        &self.system.boot
    }

    pub fn system_tpm(&self) -> Option<&TpmConfig> {
        self.system.tpm.as_ref()
    }

    pub fn system_smbios(&self) -> Option<&SmbiosConfig> {
        self.system.smbios.as_ref()
    }

    pub fn system_applesmc(&self) -> Option<&AppleSmcConfig> {
        self.system.applesmc.as_ref()
    }

    pub fn system_memory_ballooning(&self) -> Option<&BallooningConfig> {
        self.system.memory.ballooning.as_ref()
    }

    pub fn system_memory_ivshmem(&self) -> Option<&IvshmemConfig> {
        self.system.memory.ivshmem.as_ref()
    }

    pub fn options_guest_agent(&self) -> Option<&GuestAgentConfig> {
        self.options.guest_agent.as_ref()
    }

    pub fn options_qmp(&self) -> Option<&QmpConfig> {
        self.options.qmp.as_ref()
    }

    pub fn controllers_scsi(&self) -> &[ScsiControllerConfig] {
        &self.controllers.scsi
    }

    pub fn controllers_xhci(&self) -> &[XhciControllerConfig] {
        &self.controllers.xhci
    }

    pub fn controllers_sata(&self) -> &[SataControllerConfig] {
        &self.controllers.sata
    }

    pub fn host_pci(&self) -> &[HostPciConfig] {
        &self.host.pci
    }

    pub fn host_usb(&self) -> &[UsbDeviceConfig] {
        &self.host.usb
    }

    pub fn devices_input(&self) -> &[InputDeviceConfig] {
        &self.devices.input
    }

    pub fn devices_audio(&self) -> &[AudioDeviceConfig] {
        &self.devices.audio
    }

    fn load_vm_value_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<serde_yaml::Value> {
        let content = std::fs::read_to_string(path)?;
        let processed_content = Self::substitute_env_vars(&content)?;
        let vm_value: serde_yaml::Value = serde_yaml::from_str(&processed_content)?;
        Self::ensure_yaml_mapping_root(&vm_value, "VM config")?;
        Ok(vm_value)
    }

    fn build_merged_vm_value(
        vm_value: serde_yaml::Value,
        profile_dir: &str,
        profile_names: &[String],
    ) -> anyhow::Result<serde_yaml::Value> {
        let mut merged_value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        for profile_name in profile_names {
            let profile_value = Self::load_profile_value(profile_dir, profile_name)?;
            Self::merge_yaml_values(&mut merged_value, profile_value);
        }
        Self::merge_yaml_values(&mut merged_value, vm_value);
        Ok(merged_value)
    }

    fn deserialize_and_validate(mut merged_value: serde_yaml::Value) -> anyhow::Result<Self> {
        policies::apply_profile_policies(&mut merged_value)?;
        let mut config: VmConfig = serde_yaml::from_value(merged_value)?;
        config.assign_default_device_ids();
        validation::validate_config(&config)?;
        Ok(config)
    }

    fn assign_default_device_ids(&mut self) {
        let mut reserved_drive_ids = self
            .devices
            .drives
            .iter()
            .filter_map(|drive| {
                let id = drive.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<HashSet<_>>();
        for (index, drive) in self.devices.drives.iter_mut().enumerate() {
            drive.assign_default_id(index, &mut reserved_drive_ids);
        }

        let mut reserved_network_ids = self
            .devices
            .networks
            .iter()
            .filter_map(|network| {
                let id = network.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<HashSet<_>>();
        for (index, network) in self.devices.networks.iter_mut().enumerate() {
            network.assign_default_id(index, &mut reserved_network_ids);
        }
    }

    fn resolve_profile_dir() -> anyhow::Result<String> {
        let central_config = CentralConfig::load()?;
        Ok(central_config
            .locations
            .profile_dir
            .unwrap_or_else(|| DEFAULT_PROFILE_DIR.to_string()))
    }

    fn extract_profile_names(vm_value: &serde_yaml::Value) -> anyhow::Result<Vec<String>> {
        let serde_yaml::Value::Mapping(vm_map) = vm_value else {
            return Err(anyhow::anyhow!(
                "VM config must be a YAML mapping/object at the root"
            ));
        };

        let profiles_key = serde_yaml::Value::String("profiles".to_string());
        let Some(profiles_value) = vm_map.get(&profiles_key) else {
            return Ok(Vec::new());
        };

        match profiles_value {
            serde_yaml::Value::Sequence(items) => {
                let mut profile_names = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        serde_yaml::Value::String(name) if !name.trim().is_empty() => {
                            profile_names.push(name.to_string());
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "VM config field 'profiles' must contain non-empty string names"
                            ));
                        }
                    }
                }
                Ok(profile_names)
            }
            serde_yaml::Value::Null => Ok(Vec::new()),
            _ => Err(anyhow::anyhow!(
                "VM config field 'profiles' must be a list of profile names"
            )),
        }
    }

    fn load_profile_value(
        profile_dir: &str,
        profile_name: &str,
    ) -> anyhow::Result<serde_yaml::Value> {
        if profile_name.is_empty()
            || !profile_name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Err(anyhow::anyhow!(
                "Unknown profile name '{}': only [A-Za-z0-9_-] are allowed",
                profile_name
            ));
        }

        let profile_path = Path::new(profile_dir).join(format!("{}.yaml", profile_name));
        if !profile_path.exists() {
            return Err(anyhow::anyhow!(
                "Unknown profile '{}': profile file not found at '{}'",
                profile_name,
                profile_path.display()
            ));
        }

        let profile_content = std::fs::read_to_string(&profile_path).map_err(|err| {
            anyhow::anyhow!(
                "Failed to read profile '{}' from '{}': {}",
                profile_name,
                profile_path.display(),
                err
            )
        })?;
        let processed_content = Self::substitute_env_vars(&profile_content)?;
        let profile_value: serde_yaml::Value =
            serde_yaml::from_str(&processed_content).map_err(|err| {
                anyhow::anyhow!(
                    "Failed to parse profile '{}' from '{}': {}",
                    profile_name,
                    profile_path.display(),
                    err
                )
            })?;
        Self::ensure_yaml_mapping_root(&profile_value, &format!("Profile '{}'", profile_name))?;
        Ok(profile_value)
    }

    fn ensure_yaml_mapping_root(value: &serde_yaml::Value, context: &str) -> anyhow::Result<()> {
        if !matches!(value, serde_yaml::Value::Mapping(_)) {
            return Err(anyhow::anyhow!(
                "{} must be a YAML mapping/object at the root",
                context
            ));
        }
        Ok(())
    }

    fn merge_yaml_values(base: &mut serde_yaml::Value, overlay: serde_yaml::Value) {
        merge::merge_yaml_values(base, overlay);
    }

    /// Substitute environment variables in configuration content.
    /// Supports ${VAR_NAME} and $VAR_NAME syntax.
    fn substitute_env_vars(content: &str) -> anyhow::Result<String> {
        env::substitute_env_vars(content)
    }

    /// Load configuration from a YAML string.
    #[allow(dead_code)]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let processed_content = Self::substitute_env_vars(content)?;
        let vm_value: serde_yaml::Value = serde_yaml::from_str(&processed_content)?;
        Self::ensure_yaml_mapping_root(&vm_value, "VM config")?;

        let profile_names = Self::extract_profile_names(&vm_value)?;
        let profile_dir = Self::resolve_profile_dir()?;
        let merged_value = Self::build_merged_vm_value(vm_value, &profile_dir, &profile_names)?;
        Self::deserialize_and_validate(merged_value)
    }
}
