use std::{collections::HashMap, fmt::Display};

use crate::runtime_config::{Bios, Boot, StorageResource};

pub struct BootModelBuilder {}
impl BootModelBuilder {
    pub fn build(boot: &Boot, storage_resources: &HashMap<String, StorageResource>) -> Result<BootModel, String> {
        let bios = match boot.bios() {
            Bios::SeaBios { seabios: _ } => BiosModel::SeaBios(SeaBiosModel {}),
            Bios::Uefi { uefi } => {
                let uefi_resource = storage_resources.get(uefi.resource()).ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by UEFI firmware",
                        uefi.resource()
                    )
                })?;
                BiosModel::Uefi(UefiModel { storage: uefi_resource.clone() })
            },
        };
        Ok(BootModel {
            bios,
        })
    }
}

pub struct BootModel {
    bios: BiosModel,
}
impl Display for BootModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.bios {
            BiosModel::SeaBios(_) => write!(f, "SeaBios"),
            BiosModel::Uefi(uefi) => {
                write!(f, "Uefi: {:?}", uefi.storage)
            },
        }
    }
}

pub enum BiosModel {
    SeaBios(SeaBiosModel),
    Uefi(UefiModel),
}
impl Default for BiosModel {
    fn default() -> Self {
        BiosModel::SeaBios(SeaBiosModel::default())
    }
}
pub struct SeaBiosModel {}
impl Default for SeaBiosModel {
    fn default() -> Self {
        SeaBiosModel {}
    }
}
pub struct UefiModel {
    storage: StorageResource
}
//impl Default for UefiModel {       
//    fn default() -> Self {
//        UefiModel {}
//    }
//}