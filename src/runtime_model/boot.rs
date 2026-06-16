use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, Getters, new)]
pub struct Boot {
    #[serde(flatten)]
    bios: Bios,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bios {
    SeaBios { seabios: SeaBios },
    Uefi { uefi: Uefi },
}
impl Default for Bios {
    fn default() -> Self {
        Bios::SeaBios {
            seabios: SeaBios::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, Getters, new)]
pub struct SeaBios {
    firmware: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, Getters, new)]
pub struct Uefi {
    resource: String,
}
