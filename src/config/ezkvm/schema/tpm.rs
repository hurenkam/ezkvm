use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TpmSchema {
    Emulated { swtpm: SwtpmSchema },
    Passthrough { hwtpm: HwtpmSchema },
}
impl Default for TpmSchema {
    fn default() -> Self {
        TpmSchema::Emulated {
            swtpm: SwtpmSchema::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct SwtpmSchema {
    version: f32,
    resource: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct HwtpmSchema {
    resource: String,
}
