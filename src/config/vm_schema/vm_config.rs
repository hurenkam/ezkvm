use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    UsbDeviceConfig, VmOptions, VncConfig, XhciControllerConfig,
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

    /// VNC display configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vnc: Option<VncConfig>,

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
        mut vm_value: serde_yaml::Value,
        profile_dir: &str,
        profile_names: &[String],
    ) -> anyhow::Result<serde_yaml::Value> {
        Self::normalize_devices_controller_sequence(&mut vm_value);
        Self::normalize_controller_owned_devices(&mut vm_value);

        let mut merged_value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        for profile_name in profile_names {
            let profile_value = Self::load_profile_value(profile_dir, profile_name)?;
            Self::merge_yaml_values(&mut merged_value, profile_value);
        }
        Self::merge_yaml_values(&mut merged_value, vm_value);
        Ok(merged_value)
    }

    fn deserialize_and_validate(mut merged_value: serde_yaml::Value) -> anyhow::Result<Self> {
        Self::normalize_devices_controller_sequence(&mut merged_value);
        Self::normalize_controller_owned_devices(&mut merged_value);
        policies::apply_profile_policies(&mut merged_value)?;
        let mut config: VmConfig = serde_yaml::from_value(merged_value)?;
        config.assign_default_device_ids();
        validation::validate_config(&config)?;
        Ok(config)
    }

    fn normalize_devices_controller_sequence(vm_value: &mut serde_yaml::Value) {
        use serde_yaml::{Mapping, Value};

        let Value::Mapping(vm_map) = vm_value else {
            return;
        };

        let devices_key = Value::String("devices".to_string());
        let Some(devices_value) = vm_map.remove(&devices_key) else {
            return;
        };

        let mut controllers_map = match vm_map.remove(&Value::String("controllers".to_string())) {
            Some(Value::Mapping(map)) => map,
            Some(other) => {
                vm_map.insert(Value::String("controllers".to_string()), other);
                Mapping::new()
            }
            None => Mapping::new(),
        };

        match devices_value {
            Value::Mapping(mut devices_map) => {
                let nested_controllers_key = Value::String("controllers".to_string());
                if let Some(Value::Mapping(nested_controllers)) =
                    devices_map.remove(&nested_controllers_key)
                {
                    Self::merge_controller_maps(&mut controllers_map, nested_controllers);
                }
                vm_map.insert(devices_key.clone(), Value::Mapping(devices_map));
            }
            Value::Sequence(device_blocks) => {
                for block in device_blocks {
                    let Value::Mapping(mut block_map) = block else {
                        continue;
                    };

                    if let Some(Value::Mapping(legacy_map)) =
                        block_map.remove(&Value::String("legacy".to_string()))
                    {
                        for (legacy_key, legacy_value) in legacy_map {
                            let Value::String(legacy_name) = legacy_key else {
                                continue;
                            };
                            let Value::Sequence(entries) = legacy_value else {
                                continue;
                            };
                            for entry in entries {
                                Self::append_under_sequence(vm_map, "devices", &legacy_name, entry);
                            }
                        }
                        continue;
                    }

                    let controller_key = Value::String("controller".to_string());
                    let Some(Value::String(controller_name)) = block_map.remove(&controller_key)
                    else {
                        continue;
                    };

                    if controller_name == "xhci" {
                        let xhci_key = Value::String("xhci".to_string());
                        let entry = controllers_map
                            .entry(xhci_key)
                            .or_insert_with(|| Value::Sequence(Vec::new()));
                        if let Value::Sequence(items) = entry {
                            items.push(Value::Mapping(block_map));
                        }
                        continue;
                    }

                    let drives_key = Value::String("drives".to_string());
                    let Some(Value::Sequence(mut drives)) = block_map.remove(&drives_key) else {
                        continue;
                    };

                    let inferred_interface = if Self::is_scsi_controller_name(&controller_name) {
                        Some("scsi")
                    } else if Self::is_sata_controller_name(&controller_name) {
                        Some("sata")
                    } else {
                        Some(controller_name.as_str())
                    };

                    for drive in &mut drives {
                        if let Value::Mapping(drive_map) = drive {
                            let interface_key = Value::String("interface".to_string());
                            if !drive_map.contains_key(&interface_key)
                                && let Some(interface) = inferred_interface
                            {
                                drive_map
                                    .insert(interface_key, Value::String(interface.to_string()));
                            }
                        }
                    }

                    if Self::is_scsi_controller_name(&controller_name) {
                        let mut controller_map = Mapping::new();
                        controller_map.insert(
                            Value::String("type".to_string()),
                            Value::String(controller_name),
                        );
                        controller_map
                            .insert(Value::String("drives".to_string()), Value::Sequence(drives));
                        for (k, v) in block_map {
                            controller_map.insert(k, v);
                        }

                        let scsi_key = Value::String("scsi".to_string());
                        let entry = controllers_map
                            .entry(scsi_key)
                            .or_insert_with(|| Value::Sequence(Vec::new()));
                        if let Value::Sequence(items) = entry {
                            items.push(Value::Mapping(controller_map));
                        }
                    } else if Self::is_sata_controller_name(&controller_name) {
                        let mut controller_map = Mapping::new();
                        controller_map.insert(
                            Value::String("type".to_string()),
                            Value::String("ahci".to_string()),
                        );
                        controller_map
                            .insert(Value::String("drives".to_string()), Value::Sequence(drives));
                        for (k, v) in block_map {
                            controller_map.insert(k, v);
                        }

                        let sata_key = Value::String("sata".to_string());
                        let entry = controllers_map
                            .entry(sata_key)
                            .or_insert_with(|| Value::Sequence(Vec::new()));
                        if let Value::Sequence(items) = entry {
                            items.push(Value::Mapping(controller_map));
                        }
                    } else {
                        for drive in drives {
                            Self::append_under_sequence(vm_map, "devices", "drives", drive);
                        }
                    }
                }

                if !vm_map.contains_key(&devices_key) {
                    vm_map.insert(devices_key.clone(), Value::Mapping(Mapping::new()));
                }
            }
            other => {
                vm_map.insert(devices_key.clone(), other);
            }
        }

        if !controllers_map.is_empty() {
            vm_map.insert(
                Value::String("controllers".to_string()),
                Value::Mapping(controllers_map),
            );
        }
    }

    fn merge_controller_maps(target: &mut serde_yaml::Mapping, nested: serde_yaml::Mapping) {
        use serde_yaml::Value;

        for (key, value) in nested {
            match (target.get_mut(&key), value) {
                (Some(Value::Sequence(existing)), Value::Sequence(mut incoming)) => {
                    existing.append(&mut incoming);
                }
                (_, replacement) => {
                    target.insert(key, replacement);
                }
            }
        }
    }

    fn is_scsi_controller_name(name: &str) -> bool {
        matches!(
            name,
            "pvscsi" | "virtio-scsi-single" | "virtio-scsi-pci" | "lsi" | "megasas"
        )
    }

    fn is_sata_controller_name(name: &str) -> bool {
        matches!(name, "sata" | "ahci")
    }

    pub(crate) fn assign_default_device_ids(&mut self) {
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
        let mut drive_interface_indices: HashMap<String, usize> = HashMap::new();
        for drive in &mut self.devices.drives {
            let interface_key = drive.interface.trim().to_string();
            let index = drive_interface_indices
                .entry(interface_key)
                .and_modify(|v| *v += 1)
                .or_insert(0);
            drive.assign_default_id(*index, &mut reserved_drive_ids);
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

        let mut reserved_usb_ids = self
            .host
            .usb
            .iter()
            .filter_map(|usb| {
                let id = usb.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<HashSet<_>>();
        for (index, usb) in self.host.usb.iter_mut().enumerate() {
            usb.assign_default_id(index, &mut reserved_usb_ids);
        }

        let mut reserved_scsi_ids = self
            .controllers
            .scsi
            .iter()
            .filter_map(|c| {
                let id = c.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<HashSet<_>>();
        for (index, ctrl) in self.controllers.scsi.iter_mut().enumerate() {
            ctrl.assign_default_id(index, &mut reserved_scsi_ids);
        }

        let mut reserved_sata_ids = self
            .controllers
            .sata
            .iter()
            .filter_map(|c| {
                let id = c.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<HashSet<_>>();
        for (index, ctrl) in self.controllers.sata.iter_mut().enumerate() {
            ctrl.assign_default_id(index, &mut reserved_sata_ids);
        }

        let mut reserved_xhci_ids = self
            .controllers
            .xhci
            .iter()
            .filter_map(|c| {
                let id = c.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<HashSet<_>>();
        for (index, ctrl) in self.controllers.xhci.iter_mut().enumerate() {
            ctrl.assign_default_id(index, &mut reserved_xhci_ids);
        }

        let mut reserved_pci_ids = self
            .host
            .pci
            .iter()
            .filter_map(|p| {
                let id = p.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<HashSet<_>>();
        for (index, pci) in self.host.pci.iter_mut().enumerate() {
            pci.assign_default_id(index, &mut reserved_pci_ids);
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

    fn normalize_controller_owned_devices(vm_value: &mut serde_yaml::Value) {
        use serde_yaml::{Mapping, Value};

        let Value::Mapping(vm_map) = vm_value else {
            return;
        };

        let controllers_key = Value::String("controllers".to_string());
        let Some(controllers_value) = vm_map.remove(&controllers_key) else {
            return;
        };

        let mut controllers_map = match controllers_value {
            Value::Mapping(map) => map,
            other => {
                vm_map.insert(controllers_key, other);
                return;
            }
        };

        let scsi_key = Value::String("scsi".to_string());
        if let Some(Value::Sequence(scsi_controllers)) = controllers_map.get_mut(&scsi_key) {
            for (index, controller_value) in scsi_controllers.iter_mut().enumerate() {
                let Value::Mapping(controller_map) = controller_value else {
                    continue;
                };

                let controller_id = Self::ensure_controller_id(controller_map, "scsihw", index);
                let drives_key = Value::String("drives".to_string());
                let Some(drives_value) = controller_map.remove(&drives_key) else {
                    continue;
                };

                if let Value::Sequence(drives) = drives_value {
                    for mut drive_value in drives {
                        if let Value::Mapping(drive_map) = &mut drive_value {
                            let controller_key = Value::String("controller".to_string());
                            if !drive_map.contains_key(&controller_key) {
                                drive_map
                                    .insert(controller_key, Value::String(controller_id.clone()));
                            }
                            let interface_key = Value::String("interface".to_string());
                            if !drive_map.contains_key(&interface_key) {
                                drive_map.insert(interface_key, Value::String("scsi".to_string()));
                            }
                        }
                        Self::append_under_sequence(vm_map, "devices", "drives", drive_value);
                    }
                }
            }
        }

        let sata_key = Value::String("sata".to_string());
        if let Some(Value::Sequence(sata_controllers)) = controllers_map.get_mut(&sata_key) {
            for (index, controller_value) in sata_controllers.iter_mut().enumerate() {
                let Value::Mapping(controller_map) = controller_value else {
                    continue;
                };

                let controller_id = Self::ensure_controller_id(controller_map, "sata", index);
                let drives_key = Value::String("drives".to_string());
                let Some(drives_value) = controller_map.remove(&drives_key) else {
                    continue;
                };

                if let Value::Sequence(drives) = drives_value {
                    for mut drive_value in drives {
                        if let Value::Mapping(drive_map) = &mut drive_value {
                            let controller_key = Value::String("controller".to_string());
                            if !drive_map.contains_key(&controller_key) {
                                drive_map
                                    .insert(controller_key, Value::String(controller_id.clone()));
                            }
                            let interface_key = Value::String("interface".to_string());
                            if !drive_map.contains_key(&interface_key) {
                                drive_map.insert(interface_key, Value::String("sata".to_string()));
                            }
                        }
                        Self::append_under_sequence(vm_map, "devices", "drives", drive_value);
                    }
                }
            }
        }

        // IDE controllers are implicit in the chipset and have no ControllersConfig entry.
        // Consume them here: extract drives and promote them to devices.drives with interface: ide.
        let ide_key = Value::String("ide".to_string());
        if let Some(Value::Sequence(ide_controllers)) = controllers_map.remove(&ide_key) {
            for controller_value in ide_controllers {
                let Value::Mapping(mut controller_map) = controller_value else {
                    continue;
                };
                let drives_key = Value::String("drives".to_string());
                let Some(Value::Sequence(drives)) = controller_map.remove(&drives_key) else {
                    continue;
                };
                for mut drive_value in drives {
                    if let Value::Mapping(drive_map) = &mut drive_value {
                        let interface_key = Value::String("interface".to_string());
                        if !drive_map.contains_key(&interface_key) {
                            drive_map.insert(interface_key, Value::String("ide".to_string()));
                        }
                    }
                    Self::append_under_sequence(vm_map, "devices", "drives", drive_value);
                }
            }
        }

        let xhci_key = Value::String("xhci".to_string());
        if let Some(Value::Sequence(xhci_controllers)) = controllers_map.get_mut(&xhci_key) {
            for (index, controller_value) in xhci_controllers.iter_mut().enumerate() {
                let Value::Mapping(controller_map) = controller_value else {
                    continue;
                };

                let controller_id = Self::ensure_controller_id(controller_map, "xhci", index);
                let usb_key = Value::String("usb".to_string());
                let Some(usb_value) = controller_map.remove(&usb_key) else {
                    continue;
                };

                if let Value::Sequence(usb_devices) = usb_value {
                    for mut usb_device_value in usb_devices {
                        if let Value::Mapping(usb_map) = &mut usb_device_value {
                            let bus_key = Value::String("bus".to_string());
                            if !usb_map.contains_key(&bus_key) {
                                usb_map.insert(
                                    bus_key,
                                    Value::String(format!("{}.0", controller_id.as_str())),
                                );
                            }
                        }
                        Self::append_under_sequence(vm_map, "host", "usb", usb_device_value);
                    }
                }
            }
        }

        vm_map.insert(controllers_key, Value::Mapping(controllers_map));

        if !vm_map.contains_key(&Value::String("devices".to_string())) {
            vm_map.insert("devices".into(), Value::Mapping(Mapping::new()));
        }
        if !vm_map.contains_key(&Value::String("host".to_string())) {
            vm_map.insert("host".into(), Value::Mapping(Mapping::new()));
        }
    }

    fn append_under_sequence(
        vm_map: &mut serde_yaml::Mapping,
        parent: &str,
        child: &str,
        entry: serde_yaml::Value,
    ) {
        use serde_yaml::{Mapping, Value};

        let parent_key = Value::String(parent.to_string());
        if !vm_map.contains_key(&parent_key) {
            vm_map.insert(parent_key.clone(), Value::Mapping(Mapping::new()));
        }

        let Some(Value::Mapping(parent_map)) = vm_map.get_mut(&parent_key) else {
            return;
        };

        let child_key = Value::String(child.to_string());
        if !parent_map.contains_key(&child_key) {
            parent_map.insert(child_key.clone(), Value::Sequence(Vec::new()));
        }

        if let Some(Value::Sequence(items)) = parent_map.get_mut(&child_key) {
            items.push(entry);
        }
    }

    fn ensure_controller_id(
        controller_map: &mut serde_yaml::Mapping,
        prefix: &str,
        index: usize,
    ) -> String {
        use serde_yaml::Value;

        let id_key = Value::String("id".to_string());
        if let Some(Value::String(id)) = controller_map.get(&id_key)
            && !id.trim().is_empty()
        {
            return id.clone();
        }

        let generated = if prefix == "xhci" && index == 0 {
            "xhci".to_string()
        } else {
            format!("{}{}", prefix, index)
        };

        controller_map.insert(id_key, Value::String(generated.clone()));
        generated
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
