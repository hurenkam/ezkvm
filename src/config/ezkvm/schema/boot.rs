use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize, Getters, new)]
pub struct BootSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    secure: Option<bool>,
    #[serde(flatten)]
    bios: BiosSchema,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BiosSchema {
    SeaBios { seabios: SeaBiosSchema },
    Uefi { uefi: UefiSchema },
}
impl Default for BiosSchema {
    fn default() -> Self {
        BiosSchema::SeaBios {
            seabios: SeaBiosSchema::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct SeaBiosSchema {
    firmware: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct UefiSchema {
    resource: String,
}
