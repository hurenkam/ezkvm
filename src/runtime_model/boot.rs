use std::{collections::HashMap, fmt::Display};

use crate::runtime_config::{Bios, Boot, StorageResource};

const OVMF_CODE_PATH: &str = "/usr/share/pve-edk2-firmware/OVMF_CODE_4M.secboot.fd";
const OVMF_VARS_SIZE: usize = 540_672;

pub struct BootModelBuilder {}
impl BootModelBuilder {
    pub fn build(
        boot: &Boot,
        storage_resources: &HashMap<String, StorageResource>,
    ) -> Result<BootModel, String> {
        let bios = match boot.bios() {
            Bios::SeaBios { seabios: _ } => BiosModel::SeaBios(SeaBiosModel {}),
            Bios::Uefi { uefi } => {
                let uefi_resource = storage_resources.get(uefi.resource()).ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by UEFI firmware",
                        uefi.resource()
                    )
                })?;
                BiosModel::Uefi(UefiModel {
                    storage: uefi_resource.clone(),
                })
            }
        };
        Ok(BootModel { bios })
    }
}

pub struct BootModel {
    bios: BiosModel,
}
impl BootModel {
    pub fn qemu_args(&self) -> Vec<String> {
        let mut args = vec![
            "-boot".to_string(),
            "menu=on,strict=on,reboot-timeout=1000".to_string(),
        ];

        match &self.bios {
            BiosModel::SeaBios(seabios) => args.extend(seabios.qemu_args()),
            BiosModel::Uefi(uefi) => args.extend(uefi.qemu_args()),
        }

        args
    }
}
impl Display for BootModel {
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
pub struct SeaBiosModel {}
impl SeaBiosModel {
    pub fn qemu_args(&self) -> Vec<String> {
        vec!["-bios".to_string(), "/usr/share/qemu/bios.bin".to_string()]
    }
}
impl Default for SeaBiosModel {
    fn default() -> Self {
        SeaBiosModel {}
    }
}
pub struct UefiModel {
    storage: StorageResource,
}
impl UefiModel {
    pub fn qemu_args(&self) -> Vec<String> {
        let vars_path = match &self.storage {
            StorageResource::File { file } => file.clone(),
            StorageResource::BlockDevice { block_device } => block_device.clone(),
        };

        vec![
            "-drive".to_string(),
            format!(
                "if=pflash,unit=0,format=raw,readonly=on,file={}",
                OVMF_CODE_PATH
            ),
            "-drive".to_string(),
            format!(
                "if=pflash,unit=1,id=drive-efidisk0,format=raw,file={},size={}",
                vars_path, OVMF_VARS_SIZE
            ),
        ]
    }
}
//impl Default for UefiModel {
//    fn default() -> Self {
//        UefiModel {}
//    }
//}
