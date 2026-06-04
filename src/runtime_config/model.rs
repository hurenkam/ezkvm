use std::{fmt, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::runtime_config::{ValidationIssue, validation::{check_required_strings, validate_machine_consistency, validate_unique_ids_network, validate_unique_ids_resources, validate_unique_ids_storage, validate_vm_name_filename_match}};

/// Schema version for ezkvm runtime config specification.
pub const EZKVM_CONFIG_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub metadata: Metadata,
    pub virtual_machine: VirtualMachine,
}
impl RuntimeConfig {
    pub fn validate_runtime(&self, source_path: Option<&Path>) -> Result<(), String> {
        let fallback = PathBuf::from(format!("{}.yaml", self.metadata.vm_name));
        let path = source_path.unwrap_or(&fallback);

        self.validate_runtime_config(path).map_err(|error| match error.report() {
            Some(report) => {
                let formatter = super::DefaultReportFormatter::new();
                report.render_with(
                    &formatter,
                    super::ValidationReportFormat::Human,
                )
            }
            None => error.to_string(),
        })
    }

    pub fn validate_runtime_config(
        &self,
        filename: &Path,
    ) -> Result<(), super::ConformanceError> {
        let mut issues = Vec::new();

        check_required_strings(
            &mut issues,
            "metadata.schema_version",
            &self.metadata.schema_version,
        );
        check_required_strings(&mut issues, "metadata.vm_name", &self.metadata.vm_name);
        check_required_strings(
            &mut issues,
            "virtual_machine.system.machine.family",
            &self.virtual_machine.system.machine.family,
        );
        check_required_strings(
            &mut issues,
            "virtual_machine.system.machine.chipset",
            &self.virtual_machine.system.machine.chipset,
        );
        check_required_strings(
            &mut issues,
            "virtual_machine.system.cpu.model",
            &self.virtual_machine.system.cpu.model,
        );

        if self.virtual_machine.system.memory.min < 0 {
            issues.push(
                ValidationIssue::new(
                    "virtual_machine.system.memory.min",
                    "must be an integer >= 0",
                )
                .with_remediation("Change memory.min to a non-negative value"),
            );
        }

        validate_vm_name_filename_match(&mut issues, &self.metadata.vm_name, filename);
        validate_machine_consistency(
            &mut issues,
            &self.virtual_machine.system.machine.family,
            &self.virtual_machine.system.machine.chipset,
        );
        validate_unique_ids_storage(&mut issues, &self.virtual_machine.storage);
        validate_unique_ids_network(&mut issues, &self.virtual_machine.network);
        validate_unique_ids_resources(&mut issues, &self.virtual_machine.resources);

        if issues.is_empty() {
            Ok(())
        } else {
            Err(super::ConformanceError::Validation(issues.len(), issues))
        }
    }

}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub schema_version: String,
    pub vm_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VirtualMachine {
    pub system: System,
    #[serde(default)]
    pub storage: Vec<StorageEntry>,
    #[serde(default)]
    pub network: Vec<NetworkEntry>,
    #[serde(default)]
    pub resources: Vec<ResourceRef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct System {
    pub machine: Machine,
    pub cpu: Cpu,
    pub memory: Memory,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Machine {
    pub family: String,
    pub chipset: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Cpu {
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Memory {
    pub min: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageEntry {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkEntry {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResourceRef {
    pub id: String,
}

impl fmt::Display for RuntimeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = serde_yaml::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&rendered)
    }
}

// Conversion implementations for YAML parsing
impl From<String> for StorageEntry {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for NetworkEntry {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for ResourceRef {
    fn from(id: String) -> Self {
        Self { id }
    }
}
