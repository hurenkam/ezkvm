use std::{collections::HashMap, fmt::Display, sync::Arc};

use crate::runtime_config::{StorageResource, Tpm};

pub trait TpmApi: Display {
    fn qemu_args(&self, vm_name: &str) -> Vec<String>;
}
pub struct TpmModelBuilder {}
impl TpmModelBuilder {
    pub fn build(
        tpm: &Tpm,
        storage_resources: &HashMap<String, StorageResource>,
    ) -> Result<Arc<dyn TpmApi>, String> {
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
            Tpm::Passthrough { hwtpm: _ } => todo!(),
        }
    }
}
pub enum TpmModel {
    Swtpm { swtpm: SwtpmModel },
}
impl TpmApi for TpmModel {
    fn qemu_args(&self, vm_name: &str) -> Vec<String> {
        match self {
            TpmModel::Swtpm { swtpm: _ } => {
                let socket_path = format!("/var/run/ezkvm/{vm_name}.swtpm");

                let args = vec![
                    "-chardev".to_string(),
                    format!("socket,id=tpmchar,path={socket_path}"),
                    "-tpmdev".to_string(),
                    "emulator,id=tpmdev,chardev=tpmchar".to_string(),
                    "-device".to_string(),
                    "tpm-tis,tpmdev=tpmdev".to_string(),
                ];
                args
            }
        }
    }
}
pub struct SwtpmModel {
    version: f32,
    resource: StorageResource,
}
impl Display for TpmModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TpmModel::Swtpm { swtpm } => {
                write!(f, "Software TPM (v{}, {:?})", swtpm.version, swtpm.resource)
            }
        }
    }
}
