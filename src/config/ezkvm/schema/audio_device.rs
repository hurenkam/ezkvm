use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct AudioDeviceSchema {
    device_type: String,
    driver: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_device_schema_round_trips_yaml() {
        let schema = AudioDeviceSchema::new("ich9-intel-hda".to_string(), "spice".to_string());
        let yaml = crate::serde_yaml::to_string(&schema).unwrap();
        assert!(yaml.contains("device_type"), "yaml: {yaml}");
        assert!(yaml.contains("driver"), "yaml: {yaml}");
        let decoded: AudioDeviceSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.device_type(), "ich9-intel-hda");
        assert_eq!(decoded.driver(), "spice");
    }
}
