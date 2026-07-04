use crate::runtime_model::StorageResource;
use derive_getters::Getters;
use derive_new::new;

#[derive(Getters, new)]
pub struct BootModel {
    bios: BiosModel,
}
impl BootModel {}
impl std::fmt::Display for BootModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.bios {
            BiosModel::SeaBios(_) => write!(f, "SeaBios"),
            BiosModel::Uefi(uefi) => {
                write!(f, "Uefi: {:?}", uefi.storage)
            }
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
#[derive(Default)]
pub struct SeaBiosModel {}
#[derive(Getters, new)]
pub struct UefiModel {
    storage: StorageResource,
}
impl UefiModel {}
//impl Default for UefiModel {
//    fn default() -> Self {
//        UefiModel {}
//    }
//}
