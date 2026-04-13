use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use super::super::{
    CentralConfig, DEFAULT_PROFILE_DIR,
    loader::{env, merge, policies},
    validation,
};

use super::super::{
    AudioDeviceConfig, BallooningConfig, GuestAgentConfig, HostPciConfig, HypervConfig,
    InputDeviceConfig, IscsiDiskConfig, IvshmemConfig, NumaConfig, QmpConfig, ScsiControllerConfig,
    SmbiosConfig, SpiceConfig, TpmConfig, UsbDeviceConfig, VmOptions, XhciControllerConfig,
};
use super::{BootConfig, DeviceConfig, SystemConfig};

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
    #[serde(default)]
    pub profiles: Vec<String>,

    /// System configuration (CPU, memory, etc.)
    pub system: SystemConfig,

    /// Boot configuration
    #[serde(default)]
    pub boot: BootConfig,

    /// Device configuration
    #[serde(default)]
    pub devices: DeviceConfig,

    /// TPM configuration
    #[serde(default)]
    pub tpm: Option<TpmConfig>,

    /// Guest agent configuration
    #[serde(default)]
    pub guest_agent: Option<GuestAgentConfig>,

    /// Memory ballooning configuration
    #[serde(default)]
    pub ballooning: Option<BallooningConfig>,

    /// Hardware passthrough configuration
    #[serde(default)]
    pub hostpci: Vec<HostPciConfig>,

    /// USB device passthrough configuration
    #[serde(default)]
    pub usb_devices: Vec<UsbDeviceConfig>,

    /// XHCI controller configuration
    #[serde(default)]
    pub xhci_controllers: Vec<XhciControllerConfig>,

    /// SPICE display configuration
    #[serde(default)]
    pub spice: Option<SpiceConfig>,

    /// Audio devices backed by the selected audio backend
    #[serde(default)]
    pub audio_devices: Vec<AudioDeviceConfig>,

    /// Explicit input devices
    #[serde(default)]
    pub input_devices: Vec<InputDeviceConfig>,

    /// Looking Glass shared memory configuration
    #[serde(default)]
    pub ivshmem: Option<IvshmemConfig>,

    /// SCSI controller configuration
    #[serde(default)]
    pub scsi_controllers: Vec<ScsiControllerConfig>,

    /// iSCSI storage configuration
    #[serde(default)]
    pub iscsi_disks: Vec<IscsiDiskConfig>,

    /// QMP monitoring configuration
    #[serde(default)]
    pub qmp: Option<QmpConfig>,

    /// SMBIOS system information configuration
    #[serde(default)]
    pub smbios: Option<SmbiosConfig>,

    /// Legacy top-level NUMA topology configuration (migrated to system.cpu.numa).
    #[serde(rename = "numa", default)]
    legacy_numa: Vec<NumaConfig>,

    /// Hyper-V enlightenments configuration
    #[serde(default)]
    pub hyperv: Option<HypervConfig>,

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
        &self.boot
    }

    pub fn system_tpm(&self) -> Option<&TpmConfig> {
        self.tpm.as_ref()
    }

    pub fn system_smbios(&self) -> Option<&SmbiosConfig> {
        self.smbios.as_ref()
    }

    pub fn system_memory_ballooning(&self) -> Option<&BallooningConfig> {
        self.ballooning.as_ref()
    }

    pub fn system_memory_ivshmem(&self) -> Option<&IvshmemConfig> {
        self.ivshmem.as_ref()
    }

    pub fn options_guest_agent(&self) -> Option<&GuestAgentConfig> {
        self.guest_agent.as_ref()
    }

    pub fn options_qmp(&self) -> Option<&QmpConfig> {
        self.qmp.as_ref()
    }

    pub fn controllers_scsi(&self) -> &[ScsiControllerConfig] {
        &self.scsi_controllers
    }

    pub fn controllers_xhci(&self) -> &[XhciControllerConfig] {
        &self.xhci_controllers
    }

    pub fn host_pci(&self) -> &[HostPciConfig] {
        &self.hostpci
    }

    pub fn host_usb(&self) -> &[UsbDeviceConfig] {
        &self.usb_devices
    }

    pub fn devices_input(&self) -> &[InputDeviceConfig] {
        &self.input_devices
    }

    pub fn devices_audio(&self) -> &[AudioDeviceConfig] {
        &self.audio_devices
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
        Self::normalize_schema_value(&mut merged_value)?;
        policies::apply_profile_policies(&mut merged_value)?;
        let mut config: VmConfig = serde_yaml::from_value(merged_value)?;
        config.normalize_legacy_layout();
        config.assign_default_device_ids();
        validation::validate_config(&config)?;
        Ok(config)
    }

    fn normalize_schema_value(root: &mut serde_yaml::Value) -> anyhow::Result<()> {
        let serde_yaml::Value::Mapping(root_map) = root else {
            return Err(anyhow::anyhow!(
                "VM config must be a YAML mapping/object at the root"
            ));
        };

        Self::ensure_mapping_or_absent(root_map, &["system", "boot"], "system.boot")?;
        Self::ensure_mapping_or_absent(root_map, &["system", "tpm"], "system.tpm")?;
        Self::ensure_mapping_or_absent(root_map, &["system", "smbios"], "system.smbios")?;
        Self::ensure_mapping_or_absent(
            root_map,
            &["options", "guest_agent"],
            "options.guest_agent",
        )?;
        Self::ensure_mapping_or_absent(root_map, &["options", "qmp"], "options.qmp")?;

        Self::ensure_sequence_or_absent(root_map, &["controllers", "scsi"], "controllers.scsi")?;
        Self::ensure_sequence_or_absent(root_map, &["controllers", "xhci"], "controllers.xhci")?;
        Self::ensure_sequence_or_absent(root_map, &["host", "pci"], "host.pci")?;
        Self::ensure_sequence_or_absent(root_map, &["host", "usb"], "host.usb")?;
        Self::ensure_sequence_or_absent(root_map, &["devices", "input"], "devices.input")?;
        Self::ensure_sequence_or_absent(root_map, &["devices", "audio"], "devices.audio")?;

        // Target-path scalar/object aliases override legacy fields.
        Self::map_nested_to_top_level(root_map, &["system", "boot"], "boot");
        Self::map_nested_to_top_level(root_map, &["system", "tpm"], "tpm");
        Self::map_nested_to_top_level(root_map, &["system", "smbios"], "smbios");
        Self::map_nested_to_top_level(root_map, &["options", "guest_agent"], "guest_agent");
        Self::map_nested_to_top_level(root_map, &["options", "qmp"], "qmp");

        // system.memory object migration support:
        // - system.memory.size -> system.memory scalar
        // - system.memory.ballooning -> top-level ballooning
        // - system.memory.ivshmem -> top-level ivshmem
        if let Some(memory_value) = Self::get_nested_cloned(root_map, &["system", "memory"]) {
            match memory_value {
                serde_yaml::Value::Mapping(memory_map) => {
                    let size_key = serde_yaml::Value::String("size".to_string());
                    let Some(size) = memory_map.get(size_key) else {
                        return Err(anyhow::anyhow!("system.memory object must include 'size'"));
                    };
                    Self::set_nested_value(root_map, &["system", "memory"], size.clone());

                    if let Some(ballooning) =
                        memory_map.get(serde_yaml::Value::String("ballooning".to_string()))
                    {
                        root_map.insert(
                            serde_yaml::Value::String("ballooning".to_string()),
                            ballooning.clone(),
                        );
                    }
                    if let Some(ivshmem) =
                        memory_map.get(serde_yaml::Value::String("ivshmem".to_string()))
                    {
                        root_map.insert(
                            serde_yaml::Value::String("ivshmem".to_string()),
                            ivshmem.clone(),
                        );
                    }
                }
                serde_yaml::Value::Number(_) => {}
                serde_yaml::Value::Null => {}
                _ => {
                    return Err(anyhow::anyhow!(
                        "system.memory must be an integer MiB value or an object with 'size'"
                    ));
                }
            }
        }

        // For moved list fields, canonical legacy field receives legacy + target entries.
        Self::merge_moved_list(root_map, "scsi_controllers", &["controllers", "scsi"]);
        Self::merge_moved_list(root_map, "xhci_controllers", &["controllers", "xhci"]);
        Self::merge_moved_list(root_map, "hostpci", &["host", "pci"]);
        Self::merge_moved_list(root_map, "usb_devices", &["host", "usb"]);
        Self::merge_moved_list(root_map, "input_devices", &["devices", "input"]);
        Self::merge_moved_list(root_map, "audio_devices", &["devices", "audio"]);

        Ok(())
    }

    fn ensure_mapping_or_absent(
        root_map: &serde_yaml::Mapping,
        path: &[&str],
        path_label: &str,
    ) -> anyhow::Result<()> {
        if let Some(value) = Self::get_nested_cloned(root_map, path)
            && !matches!(
                value,
                serde_yaml::Value::Mapping(_) | serde_yaml::Value::Null
            )
        {
            return Err(anyhow::anyhow!("{} must be an object", path_label));
        }
        Ok(())
    }

    fn ensure_sequence_or_absent(
        root_map: &serde_yaml::Mapping,
        path: &[&str],
        path_label: &str,
    ) -> anyhow::Result<()> {
        if let Some(value) = Self::get_nested_cloned(root_map, path)
            && !matches!(
                value,
                serde_yaml::Value::Sequence(_) | serde_yaml::Value::Null
            )
        {
            return Err(anyhow::anyhow!("{} must be a list", path_label));
        }
        Ok(())
    }

    fn map_nested_to_top_level(
        root_map: &mut serde_yaml::Mapping,
        nested_path: &[&str],
        top_level_key: &str,
    ) {
        if let Some(value) = Self::get_nested_cloned(root_map, nested_path) {
            root_map.insert(serde_yaml::Value::String(top_level_key.to_string()), value);
        }
    }

    fn merge_moved_list(
        root_map: &mut serde_yaml::Mapping,
        legacy_key: &str,
        target_path: &[&str],
    ) {
        let legacy_entries = match root_map.get(serde_yaml::Value::String(legacy_key.to_string())) {
            Some(serde_yaml::Value::Sequence(items)) => items.clone(),
            _ => Vec::new(),
        };

        let target_entries = match Self::get_nested_cloned(root_map, target_path) {
            Some(serde_yaml::Value::Sequence(items)) => items,
            _ => Vec::new(),
        };

        if legacy_entries.is_empty() && target_entries.is_empty() {
            return;
        }

        let mut merged = Vec::with_capacity(legacy_entries.len() + target_entries.len());
        merged.extend(legacy_entries);
        merged.extend(target_entries);
        root_map.insert(
            serde_yaml::Value::String(legacy_key.to_string()),
            serde_yaml::Value::Sequence(merged),
        );
    }

    fn get_nested_cloned(
        root_map: &serde_yaml::Mapping,
        path: &[&str],
    ) -> Option<serde_yaml::Value> {
        let mut current = serde_yaml::Value::Mapping(root_map.clone());

        for key in path {
            let serde_yaml::Value::Mapping(map) = current else {
                return None;
            };
            let key_value = serde_yaml::Value::String((*key).to_string());
            current = map.get(&key_value)?.clone();
        }

        Some(current)
    }

    fn set_nested_value(
        root_map: &mut serde_yaml::Mapping,
        path: &[&str],
        value: serde_yaml::Value,
    ) {
        if path.is_empty() {
            return;
        }

        if path.len() == 1 {
            root_map.insert(serde_yaml::Value::String(path[0].to_string()), value);
            return;
        }

        let mut current = root_map;
        for key in &path[..path.len() - 1] {
            let key_value = serde_yaml::Value::String((*key).to_string());
            let entry = current
                .entry(key_value)
                .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

            if !matches!(entry, serde_yaml::Value::Mapping(_)) {
                *entry = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
            }

            let serde_yaml::Value::Mapping(map) = entry else {
                return;
            };
            current = map;
        }

        current.insert(
            serde_yaml::Value::String(path[path.len() - 1].to_string()),
            value,
        );
    }

    fn normalize_legacy_layout(&mut self) {
        let legacy_numa = std::mem::take(&mut self.legacy_numa);
        self.system.normalize_cpu_fields(legacy_numa);
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
        let mut vm_value: serde_yaml::Value = serde_yaml::from_str(&processed_content)?;
        Self::ensure_yaml_mapping_root(&vm_value, "VM config")?;
        Self::normalize_schema_value(&mut vm_value)?;
        let mut config: VmConfig = serde_yaml::from_value(vm_value)?;
        config.normalize_legacy_layout();
        config.assign_default_device_ids();

        validation::validate_config(&config)?;

        Ok(config)
    }
}
