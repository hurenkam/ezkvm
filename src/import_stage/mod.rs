//! Import stage module scaffold.
//!
//! Owns source-adapter orchestration and source -> vm_spec mapping boundaries.

use std::path::Path;

use crate::vm_spec::{CanonicalDocument, ConformanceError, validate_canonical_yaml};

pub mod proxmox_conf;
pub use proxmox_conf::{ProxmoxConfImportError, ProxmoxConfImportStage};

#[derive(Debug, Clone, Copy)]
pub struct ImportRequest<'a> {
    pub source_text: &'a str,
    pub source_name: &'a Path,
}

pub trait ImportStage {
    type Error;

    fn import(&self, request: ImportRequest<'_>) -> Result<CanonicalDocument, Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum ImportStageError {
    #[error(transparent)]
    Conformance(#[from] ConformanceError),
    #[error(transparent)]
    ProxmoxConf(#[from] ProxmoxConfImportError),
}

#[derive(Debug, Default)]
pub struct CanonicalYamlImportStage;

impl ImportStage for CanonicalYamlImportStage {
    type Error = ImportStageError;

    fn import(&self, request: ImportRequest<'_>) -> Result<CanonicalDocument, Self::Error> {
        validate_canonical_yaml(request.source_text, request.source_name).map_err(Into::into)
    }
}

impl ImportStage for ProxmoxConfImportStage {
    type Error = ImportStageError;

    fn import(&self, request: ImportRequest<'_>) -> Result<CanonicalDocument, Self::Error> {
        ProxmoxConfImportStage::parse(request).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        CanonicalYamlImportStage, ImportRequest, ImportStage, ImportStageError,
    };

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
