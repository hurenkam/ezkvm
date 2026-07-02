//! ezkvm source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

mod runtime_builder;
mod schema;
mod schema_builder;
mod store;

pub use runtime_builder::EzkvmRuntimeBuilder;
pub use schema::EzkvmConfigSchema;
#[allow(unused_imports)]
pub use schema::{Bios, Boot, Device, Machine, VirtualMachine};
pub use schema_builder::EzkvmSchemaBuilder;
pub use store::EzkvmConfigFileStore;
