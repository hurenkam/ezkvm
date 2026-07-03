//! libvirt XML source and destination support for the configuration format pipeline.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/linux/

mod loader;
mod saver;

pub use loader::{LibvirtInputArgs, LibvirtLoader};
pub use saver::{LibvirtOutputArgs, LibvirtSaver};
