use std::path::Path;

use crate::vm_spec::{ConformanceError, validate_runtime_config};

use super::RuntimeConfig;
use super::diagnostics::enrich_validation_issues;
use super::parsing::parse_ezkvm_config_from_yaml;

pub(crate) fn validate_ezkvm_config(
    yaml: &str,
    filename: &Path,
) -> Result<RuntimeConfig, ConformanceError> {
    let doc = parse_ezkvm_config_from_yaml(yaml)?;
    match validate_runtime_config(&doc, filename) {
        Ok(()) => {}
        Err(ConformanceError::Validation(_, issues)) => {
            let issues = enrich_validation_issues(yaml, issues);
            return Err(ConformanceError::Validation(issues.len(), issues));
        }
        Err(other) => return Err(other),
    }
    Ok(doc)
}
