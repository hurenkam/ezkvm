//! Generic stage traits for the configuration format pipeline.
//!
//! Each adapter implements these four traits to form a symmetric four-stage pipeline:
//!
//! - **Import path**: `Parser` (text → schema) → `RuntimeBuilder` (schema → RuntimeModel)
//! - **Export path**: `SchemaBuilder` (RuntimeModel → schema) → `Marshaler` (schema → text)
//!
//! Context specific to an adapter (e.g. host path, storage config) is stored as
//! struct fields so the trait methods themselves remain uniform across adapters.

use crate::runtime_model::RuntimeModel;

/// Parses text input into a format-specific schema type.
#[allow(dead_code)] // TODO: wire to CLI
pub trait Parser {
    type Schema;
    type Error;
    fn parse(&self, source: &str) -> Result<Self::Schema, Self::Error>;
}

/// Builds a canonical `RuntimeModel` from a format-specific schema.
pub trait RuntimeBuilder {
    type Schema;
    fn with_schema(self, schema: Self::Schema) -> Self;
    fn build(self) -> Result<RuntimeModel, String>;
}

/// Builds a format-specific schema from a canonical `RuntimeModel`.
pub trait SchemaBuilder {
    type Schema;
    fn with_runtime(self, runtime: RuntimeModel) -> Self;
    fn build(self) -> Result<Self::Schema, String>;
}

/// Marshals a format-specific schema into text output.
#[allow(dead_code)] // TODO: wire to CLI
pub trait Marshaler {
    type Schema;
    type Error;
    fn marshal(&self, schema: &Self::Schema) -> Result<String, Self::Error>;
}
