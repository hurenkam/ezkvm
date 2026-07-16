use crate::config::ezkvm::schema::{ChipsetSchema, I440FXChipsetSchema, Q35ChipsetSchema};
use crate::config::ezkvm::{RootDeviceHandler, SchemaBuilder};
use crate::runtime::{Chipset, RootDevice};

#[derive(Debug, Clone, Copy, Default)]
pub struct EzkvmChipsetHandler;
impl RootDeviceHandler for EzkvmChipsetHandler {
    fn handle(&self, builder: &mut SchemaBuilder, device: &dyn RootDevice) -> Result<(), ()> {
        if let Some(chipset) = device.as_any().downcast_ref::<crate::runtime::Chipset>() {
            builder.with_chipset(chipset.clone().into());
            Ok(())
        } else {
            Err(())
        }
    }
}

impl From<Chipset> for ChipsetSchema {
    fn from(chipset: Chipset) -> Self {
        match chipset {
            Chipset::Q35(_) => ChipsetSchema::Q35 {
                q35: Q35ChipsetSchema::new(None),
            },
            Chipset::I440FX => ChipsetSchema::I440FX {
                i440fx: I440FXChipsetSchema::new(None),
            },
        }
    }
}
