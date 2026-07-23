use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::config::ezkvm::schema::{HostSchema, MetadataSchema, VirtualMachineSchema};

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct ConfigSchema {
    metadata: MetadataSchema,
    host: HostSchema,
    virtual_machine: VirtualMachineSchema,
}
