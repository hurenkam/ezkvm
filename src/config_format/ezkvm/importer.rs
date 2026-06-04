use serde_yaml::from_str;
use std::path::Path;

use crate::config_format::{EzkvmImporter, ImportError, ImportOptions, Importer, RuntimeConfig};
use crate::runtime_config::{ConformanceError, ParseError};

use super::diagnostics::enrich_validation_issues;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EzkvmInputArgs {
    #[serde(rename = "input.host")]
    pub input_host: String,
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

pub(crate) fn validate_ezkvm_config(
    yaml: &str,
    filename: &Path,
) -> Result<RuntimeConfig, ConformanceError> {
    let doc = from_str::<RuntimeConfig>(yaml).map_err(ParseError::from)?;
    match doc.validate_runtime_config(filename) {
        Ok(()) => {}
        Err(ConformanceError::Validation(_, issues)) => {
            let issues = enrich_validation_issues(yaml, issues);
            return Err(ConformanceError::Validation(issues.len(), issues));
        }
        Err(other) => return Err(other),
    }
    Ok(doc)
}

impl Importer for EzkvmImporter {
    fn import(&self, args: ImportOptions) -> Result<RuntimeConfig, ImportError> {
        let (host_path, vm_path) = match args {
            ImportOptions::Ezkvm { host, vm } => (host, vm),
            _ => return Err(ImportError::InvalidFormat),
        };

        let _host_path = host_path;
        let source_text = std::fs::read_to_string(&vm_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", vm_path, e)))?;

        validate_ezkvm_config(&source_text, Path::new(&vm_path))
            .map_err(|e| ImportError::ImportFailed(e.to_string()))
    }
}
