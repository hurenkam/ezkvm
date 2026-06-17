use std::{collections::HashMap, fmt::Display, sync::Arc};

use crate::runtime_config::{StorageResource, Tpm};

pub trait TpmApi: Display {
    fn qemu_args(&self) -> Vec<String>;    
}
pub struct TpmModelBuilder {}
impl TpmModelBuilder {
    pub fn build(tpm: &Tpm, storage_resources: &HashMap<String, StorageResource>) -> Result<Arc<dyn TpmApi>, String> {
        match tpm {
            Tpm::Emulated { swtpm } => {
                let resource = storage_resources.get(swtpm.resource()).ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by TPM",
                        swtpm.resource()
                    )
                })?;
                Ok(Arc::new(TpmModel::Swtpm {
                    swtpm: SwtpmModel {
                        version: swtpm.version().clone(),
                        resource: resource.clone(),
                    },
                }))
            }
            Tpm::Passthrough { hwtpm } => todo!(),
        }
    }
}
pub enum TpmModel {
    Swtpm { swtpm: SwtpmModel }
}
impl TpmApi for TpmModel {
    fn qemu_args(&self) -> Vec<String> {
        // For now, we only support a single TPM model, so we return an empty vector.
        todo!()
    }
}
pub struct SwtpmModel {
    version: f32,
    resource: StorageResource,
}
impl Display for TpmModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TpmModel::Swtpm { swtpm } => write!(f, "Software TPM (v{}, {:?})", swtpm.version, swtpm.resource),
        }
    }
}
