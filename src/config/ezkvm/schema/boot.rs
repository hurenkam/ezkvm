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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    efitype: Option<String>,
    #[serde(default)]
    pre_enrolled_keys: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ms_cert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    logical_size: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uefi_schema_round_trips_yaml() {
        let full = UefiSchema::new(
            "pool:vol".to_string(),
            Some("4m".to_string()),
            true,
            Some("2023".to_string()),
            Some("4M".to_string()),
        );
        let yaml = crate::serde_yaml::to_string(&full).unwrap();
        let decoded: UefiSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.resource(), "pool:vol");
        assert_eq!(decoded.efitype().as_deref(), Some("4m"));
        assert!(decoded.pre_enrolled_keys());
        assert_eq!(decoded.ms_cert().as_deref(), Some("2023"));
        assert_eq!(decoded.logical_size().as_deref(), Some("4M"));

        let minimal = UefiSchema::new("pool:vol".to_string(), None, false, None, None);
        let yaml2 = crate::serde_yaml::to_string(&minimal).unwrap();
        assert!(!yaml2.contains("efitype"), "efitype should be omitted");
        assert!(!yaml2.contains("ms_cert"), "ms_cert should be omitted");
        assert!(!yaml2.contains("logical_size"), "logical_size should be omitted");
        let decoded2: UefiSchema = crate::serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(decoded2.resource(), "pool:vol");
    }
}
