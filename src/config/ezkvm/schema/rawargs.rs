use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawArgsSchema {
    pub value: String,
}

impl RawArgsSchema {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_args_schema_round_trips_verbatim() {
        let original = "-device virtio-serial-pci -chardev spicevmc,id=vdagent";
        let schema = RawArgsSchema::new(original.to_string());
        let yaml = crate::serde_yaml::to_string(&schema).unwrap();
        let decoded: RawArgsSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.value(), original);
    }
}
