use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;
use serde::{Deserialize, Serialize};

use crate::runtime_model::StorageResource;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Tpm {
    Emulated { swtpm: Swtpm },
    Passthrough { hwtpm: Hwtpm },
}
impl Default for Tpm {
    fn default() -> Self {
        Tpm::Emulated {
            swtpm: Swtpm::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct Swtpm {
    version: f32,
    resource: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct Hwtpm {
    resource: String,
}

pub trait TpmApi: Display {
    fn swtpm_version(&self) -> Option<f32> {
        None
    }

    fn storage_resource(&self) -> Option<&StorageResource> {
        None
    }

    fn qemu_args(&self, vm_name: &str) -> Vec<String>;
}
pub struct TpmModelBuilder {}
impl TpmModelBuilder {
    pub fn build(
        tpm: Tpm,
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
                        version: *swtpm.version(),
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
    fn swtpm_version(&self) -> Option<f32> {
        match self {
            TpmModel::Swtpm { swtpm } => Some(swtpm.version),
        }
    }

    fn storage_resource(&self) -> Option<&StorageResource> {
        match self {
            TpmModel::Swtpm { swtpm } => Some(&swtpm.resource),
        }
    }

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
