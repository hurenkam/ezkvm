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
    version: String,
    resource: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct HwtpmSchema {
    resource: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swtpm_schema_round_trips_yaml() {
        let schema = SwtpmSchema::new("v2.0".to_string(), "vm1-pool:vm-108-tpmstate".to_string());
        let yaml = crate::serde_yaml::to_string(&schema).unwrap();
        let decoded: SwtpmSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.version(), "v2.0");
        assert_eq!(decoded.resource(), "vm1-pool:vm-108-tpmstate");
    }
}
