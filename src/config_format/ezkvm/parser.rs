//! Parser stage: ezkvm YAML text → `EzkvmConfigSchema`.

use crate::config_format::{
    ezkvm::{EzkvmConfigSchema, ParseError},
    stages::Parser,
};

/// Parses ezkvm YAML text into `EzkvmConfigSchema`.
pub struct EzkvmParser;

impl Parser for EzkvmParser {
    type Schema = EzkvmConfigSchema;
    type Error = ParseError;

    fn parse(&self, source: &str) -> Result<EzkvmConfigSchema, ParseError> {
        serde_yaml::from_str::<EzkvmConfigSchema>(source).map_err(ParseError::from)
    }
}
