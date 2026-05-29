//! Import stage module scaffold.
//!
//! Owns source-adapter orchestration and source -> vm_spec mapping boundaries.

use std::ffi::OsStr;
use std::path::Path;

use crate::vm_spec::{CanonicalDocument, ConformanceError, validate_canonical_yaml};

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

#[derive(Debug, thiserror::Error)]
pub enum ProxmoxConfImportError {
    #[error("proxmox .conf source is not implemented yet for {source_name}")]
    NotImplemented { source_name: String },
    #[error("expected a Proxmox .conf source name, got {source_name}")]
    InvalidSourceName { source_name: String },
}

#[derive(Debug, Default)]
pub struct CanonicalYamlImportStage;

impl ImportStage for CanonicalYamlImportStage {
    type Error = ImportStageError;

    fn import(&self, request: ImportRequest<'_>) -> Result<CanonicalDocument, Self::Error> {
        validate_canonical_yaml(request.source_text, request.source_name).map_err(Into::into)
    }
}

#[derive(Debug, Default)]
pub struct ProxmoxConfImportStage;

impl ProxmoxConfImportStage {
    pub fn new() -> Self {
        Self
    }

    fn is_proxmox_conf_source(source_name: &Path) -> bool {
        matches!(
            source_name.extension().and_then(OsStr::to_str),
            Some("conf")
        )
    }
}

impl ImportStage for ProxmoxConfImportStage {
    type Error = ImportStageError;

    fn import(&self, request: ImportRequest<'_>) -> Result<CanonicalDocument, Self::Error> {
        let source_name = request.source_name.to_string_lossy().into_owned();

        if !Self::is_proxmox_conf_source(request.source_name) {
            return Err(ProxmoxConfImportError::InvalidSourceName { source_name }.into());
        }

        let _ = request.source_text;
        Err(ProxmoxConfImportError::NotImplemented { source_name }.into())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        CanonicalYamlImportStage, ImportRequest, ImportStage, ImportStageError,
        ProxmoxConfImportError, ProxmoxConfImportStage,
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

    #[test]
    fn proxmox_conf_import_stage_is_plugged_into_the_same_trait_shape() {
        let stage = ProxmoxConfImportStage::new();
        let request = ImportRequest {
            source_text: "vmid: 100\nname: win11-dev\n",
            source_name: Path::new("/tmp/100.conf"),
        };

        let err = stage
            .import(request)
            .expect_err("skeleton should not parse yet");

        assert!(matches!(
            err,
            ImportStageError::ProxmoxConf(ProxmoxConfImportError::NotImplemented { .. })
        ));
    }
}
