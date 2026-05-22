use crate::config::{VmConfig, validation};
use crate::import::common::q35_placement::validate_placement_conflicts;

pub fn validate_generated_vm_yaml(canonical_yaml: &str) -> Result<(), String> {
    let config = VmConfig::from_str(canonical_yaml).map_err(|e| {
        format!("generated canonical YAML failed to deserialize or merge profiles: {e}")
    })?;

    validation::validate_config(&config)
        .map_err(|e| format!("generated canonical YAML failed validation: {e}"))?;

    validate_placement_conflicts(&config)
        .map_err(|e| format!("generated canonical YAML failed placement validation: {e}"))
}
