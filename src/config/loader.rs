use super::{
    CentralConfig, DEFAULT_CENTRAL_CONFIG_PATHS, DEFAULT_PROFILE_DIR, VmConfig, validation,
};
use std::collections::HashMap;
use std::path::Path;

impl VmConfig {
    /// Load configuration from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let processed_content = Self::substitute_env_vars(&content)?;
        let vm_value: serde_yaml::Value = serde_yaml::from_str(&processed_content)?;
        Self::ensure_yaml_mapping_root(&vm_value, "VM config")?;

        let profile_names = Self::extract_profile_names(&vm_value)?;
        let profile_dir = Self::resolve_profile_dir()?;

        let mut merged_value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        for profile_name in &profile_names {
            let profile_value = Self::load_profile_value(&profile_dir, profile_name)?;
            Self::merge_yaml_values(&mut merged_value, profile_value);
        }
        Self::merge_yaml_values(&mut merged_value, vm_value);

        let config: VmConfig = serde_yaml::from_value(merged_value)?;

        // Validate the configuration
        validation::validate_config(&config)?;

        Ok(config)
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
        Self::merge_yaml_values_at_path(base, overlay, &[]);
    }

    fn merge_yaml_values_at_path(
        base: &mut serde_yaml::Value,
        overlay: serde_yaml::Value,
        path: &[String],
    ) {
        match (base, overlay) {
            (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
                for (key, overlay_value) in overlay_map {
                    let key_name = match &key {
                        serde_yaml::Value::String(name) => Some(name.clone()),
                        _ => None,
                    };

                    if let Some(base_value) = base_map.get_mut(&key) {
                        let child_path = if let Some(name) = key_name {
                            let mut p = path.to_vec();
                            p.push(name);
                            p
                        } else {
                            path.to_vec()
                        };

                        if Self::is_id_merge_list_path(&child_path)
                            && matches!(base_value, serde_yaml::Value::Sequence(_))
                            && matches!(overlay_value, serde_yaml::Value::Sequence(_))
                        {
                            Self::merge_sequence_of_mappings_by_id(
                                base_value,
                                overlay_value,
                                &child_path,
                            );
                        } else if Self::is_append_unique_list_path(&child_path)
                            && matches!(base_value, serde_yaml::Value::Sequence(_))
                            && matches!(overlay_value, serde_yaml::Value::Sequence(_))
                        {
                            Self::merge_sequence_append_unique(
                                base_value,
                                overlay_value,
                                &child_path,
                            );
                        } else {
                            Self::merge_yaml_values_at_path(base_value, overlay_value, &child_path);
                        }
                    } else {
                        base_map.insert(key, overlay_value);
                    }
                }
            }
            (base_value, overlay_value) => {
                *base_value = overlay_value;
            }
        }
    }

    fn is_id_merge_list_path(path: &[String]) -> bool {
        matches!(path, [one] if one == "hostpci")
            || matches!(path, [first, second] if first == "devices" && second == "drives")
            || matches!(path, [first, second] if first == "devices" && second == "networks")
            || matches!(path, [one] if one == "usb_devices")
            || matches!(path, [one] if one == "scsi_controllers")
            || matches!(path, [one] if one == "xhci_controllers")
            || matches!(path, [one] if one == "audio_devices")
    }

    fn is_append_unique_list_path(path: &[String]) -> bool {
        matches!(path, [first, second] if first == "system" && second == "cpu_features")
            || matches!(path, [first, second] if first == "system" && second == "machine_options")
            || matches!(path, [first, second] if first == "options" && second == "global_options")
    }

    fn merge_sequence_of_mappings_by_id(
        base: &mut serde_yaml::Value,
        overlay: serde_yaml::Value,
        path: &[String],
    ) {
        let (serde_yaml::Value::Sequence(base_seq), serde_yaml::Value::Sequence(mut overlay_seq)) =
            (base, overlay)
        else {
            return;
        };

        if base_seq
            .iter()
            .any(|item| Self::yaml_mapping_id(item).is_none())
            || overlay_seq
                .iter()
                .any(|item| Self::yaml_mapping_id(item).is_none())
        {
            *base_seq = overlay_seq;
            return;
        }

        let mut index_by_id: HashMap<String, usize> = HashMap::new();
        for (idx, item) in base_seq.iter().enumerate() {
            let id = Self::yaml_mapping_id(item).unwrap();
            index_by_id.insert(id, idx);
        }

        for overlay_item in overlay_seq.drain(..) {
            let id = Self::yaml_mapping_id(&overlay_item).unwrap();

            if let Some(base_idx) = index_by_id.get(&id).copied() {
                if let Some(base_item) = base_seq.get_mut(base_idx) {
                    Self::merge_yaml_values_at_path(base_item, overlay_item, path);
                }
            } else {
                let next_idx = base_seq.len();
                base_seq.push(overlay_item);
                index_by_id.insert(id, next_idx);
            }
        }
    }

    fn yaml_mapping_id(value: &serde_yaml::Value) -> Option<String> {
        let serde_yaml::Value::Mapping(map) = value else {
            return None;
        };
        let id_key = serde_yaml::Value::String("id".to_string());
        match map.get(&id_key) {
            Some(serde_yaml::Value::String(id)) if !id.trim().is_empty() => Some(id.clone()),
            _ => None,
        }
    }

    fn yaml_mapping_name(value: &serde_yaml::Value) -> Option<String> {
        let serde_yaml::Value::Mapping(map) = value else {
            return None;
        };
        let name_key = serde_yaml::Value::String("name".to_string());
        match map.get(&name_key) {
            Some(serde_yaml::Value::String(name)) if !name.trim().is_empty() => Some(name.clone()),
            _ => None,
        }
    }

    fn merge_sequence_append_unique(
        base: &mut serde_yaml::Value,
        overlay: serde_yaml::Value,
        _path: &[String],
    ) {
        let (serde_yaml::Value::Sequence(base_seq), serde_yaml::Value::Sequence(overlay_seq)) =
            (base, overlay)
        else {
            return;
        };

        for overlay_item in overlay_seq {
            let already_present = match &overlay_item {
                serde_yaml::Value::String(s) => base_seq
                    .iter()
                    .any(|existing| matches!(existing, serde_yaml::Value::String(es) if es == s)),
                serde_yaml::Value::Mapping(_) => {
                    if let Some(overlay_name) = Self::yaml_mapping_name(&overlay_item) {
                        base_seq.iter().any(|existing| {
                            Self::yaml_mapping_name(existing)
                                .map(|name| name == overlay_name)
                                .unwrap_or(false)
                        })
                    } else {
                        base_seq.iter().any(|existing| existing == &overlay_item)
                    }
                }
                _ => base_seq.iter().any(|existing| existing == &overlay_item),
            };

            if !already_present {
                base_seq.push(overlay_item);
            }
        }
    }

    /// Substitute environment variables in configuration content
    /// Supports ${VAR_NAME} and $VAR_NAME syntax
    fn substitute_env_vars(content: &str) -> anyhow::Result<String> {
        let mut result = content.to_string();

        // Find all ${VAR} patterns
        let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
        let mut replacements = Vec::new();

        for cap in re.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str();

            match std::env::var(var_name) {
                Ok(value) => {
                    replacements.push((full_match.as_str().to_string(), value));
                }
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "Environment variable '{}' not found",
                        var_name
                    ));
                }
            }
        }

        // Apply replacements
        for (pattern, value) in replacements {
            result = result.replace(&pattern, &value);
        }

        // Also handle $VAR syntax (simple case)
        let re_simple = regex::Regex::new(r"\$([A-Z_][A-Z0-9_]*)").unwrap();
        let mut replacements_simple = Vec::new();

        for cap in re_simple.captures_iter(&result) {
            let full_match = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str();

            // Skip if it's part of a ${VAR} pattern that was already processed
            if result.contains(&format!("${{{}}}", var_name)) {
                continue;
            }

            match std::env::var(var_name) {
                Ok(value) => {
                    replacements_simple.push((full_match.as_str().to_string(), value));
                }
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "Environment variable '{}' not found",
                        var_name
                    ));
                }
            }
        }

        // Apply simple replacements
        for (pattern, value) in replacements_simple {
            result = result.replace(&pattern, &value);
        }

        Ok(result)
    }

    /// Load configuration from a YAML string
    #[allow(dead_code)]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let processed_content = Self::substitute_env_vars(content)?;
        let config: VmConfig = serde_yaml::from_str(&processed_content)?;

        // Validate the configuration
        validation::validate_config(&config)?;

        Ok(config)
    }
}

impl CentralConfig {
    /// Load central configuration from the default location or environment variable
    pub fn load() -> anyhow::Result<Self> {
        if let Ok(config_path) = std::env::var("EZKVM_CONFIG") {
            return if std::path::Path::new(&config_path).exists() {
                Self::from_file(&config_path)
            } else {
                Ok(Self::default())
            };
        }

        for config_path in DEFAULT_CENTRAL_CONFIG_PATHS {
            if std::path::Path::new(config_path).exists() {
                return Self::from_file(config_path);
            }
        }

        Ok(Self::default())
    }

    /// Load central configuration from a specific file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: CentralConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
