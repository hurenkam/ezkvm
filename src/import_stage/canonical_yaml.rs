//! Canonical YAML import adapter.
//!
//! Owns YAML → canonical VM spec transformation via direct YAML validation.

use crate::vm_spec::{CanonicalDocument, validate_canonical_yaml};

use super::{ImportRequest, ImportStage, ImportStageError};

#[derive(Debug, Default)]
pub struct CanonicalYamlImportStage;

impl ImportStage for CanonicalYamlImportStage {
    type Error = ImportStageError;

    fn import(&self, request: ImportRequest<'_>) -> Result<CanonicalDocument, Self::Error> {
        validate_canonical_yaml(request.source_text, request.source_name).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::import_stage::{ImportRequest, ImportStage, ImportStageError};

    use super::CanonicalYamlImportStage;

    fn valid_canonical_yaml() -> &'static str {
        r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 8192
"#
    }

    fn import_via_trait_object(
        stage: &dyn ImportStage<Error = ImportStageError>,
        request: ImportRequest<'_>,
    ) -> Result<crate::vm_spec::CanonicalDocument, ImportStageError> {
        stage.import(request)
    }

    #[test]
    fn canonical_yaml_import_stage_works_through_trait_object() {
        let stage = CanonicalYamlImportStage;
        let request = ImportRequest {
            source_text: valid_canonical_yaml(),
            source_name: Path::new("/tmp/win11-dev.yaml"),
        };

        let result = import_via_trait_object(&stage, request);

        assert!(result.is_ok());
    }
}
