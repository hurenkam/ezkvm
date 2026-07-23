use std::fmt::{Display as FmtDisplay, Formatter};
use crate::config::ezkvm::ConfigSchema;
use crate::serde_yaml;
use crate::config::ezkvm::file::compact_yaml::ToStyledYaml;
use crate::config::ezkvm::file::compact_yaml::emit_styled_yaml;

impl FmtDisplay for ConfigSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match serde_yaml::to_string(self) {
            Ok(serialized) => f.write_str(&serialized),
            Err(_) => f.write_str("--- failed to serialize ---"),
        }
    }
}

impl ConfigSchema {
    pub fn to_styled_compact_yaml(&self) -> Result<String, String> {
        let config_styled = self.to_styled_yaml();
        emit_styled_yaml(&config_styled)
    }
}
