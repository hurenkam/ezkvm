//! YAML parsing for ezkvm config.
//!
//! This module owns serde-based runtime config deserialization only.

use serde_yaml::from_str;

use crate::vm_spec::{ParseError, RuntimeConfig};

pub(super) fn parse_ezkvm_config_from_yaml(yaml: &str) -> Result<RuntimeConfig, ParseError> {
    from_str::<RuntimeConfig>(yaml).map_err(ParseError::from)
}
