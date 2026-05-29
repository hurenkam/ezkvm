//! Import stage module scaffold.
//!
//! Owns source-adapter orchestration and source -> vm_spec mapping boundaries.

use std::path::Path;

use crate::vm_spec::{CanonicalDocument, ConformanceError};

pub mod canonical_yaml;
pub mod proxmox_conf;

pub use canonical_yaml::CanonicalYamlImportStage;
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

impl ImportStage for ProxmoxConfImportStage {
    type Error = ImportStageError;

    fn import(&self, request: ImportRequest<'_>) -> Result<CanonicalDocument, Self::Error> {
        ProxmoxConfImportStage::parse(request).map_err(Into::into)
    }
}
