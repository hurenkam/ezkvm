mod builder;
#[allow(clippy::module_inception)]
mod schema;

pub use builder::ProxmoxSchemaBuilder;
pub use schema::{
    ProxmoxCompoundValue, ProxmoxConfigSchema, ProxmoxOption, ProxmoxSection, ProxmoxValue,
};
