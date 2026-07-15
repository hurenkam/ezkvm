use crate::{
    ezkvm::{EzkvmDeviceHandler, EzkvmSchemaBuilder},
    runtime::{Chipset, Q35Chipset, RootDevice},
};
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub struct EzkvmChipsetHandler;
impl EzkvmDeviceHandler for EzkvmChipsetHandler {
    fn handlers() -> HashMap<TypeId, fn(&mut EzkvmSchemaBuilder, &dyn RootDevice) -> Result<(), ()>>
    {
        HashMap::from([(
            TypeId::of::<Chipset>(),
            chipset_handler as fn(&mut EzkvmSchemaBuilder, &dyn RootDevice) -> Result<(), ()>,
        )])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Default)]
pub enum EzkvmChipset {
    #[default]
    Q35,
    I440FX,
}

impl From<Chipset> for EzkvmChipset {
    fn from(chipset: Chipset) -> Self {
        match chipset {
            Chipset::Q35(_) => EzkvmChipset::Q35,
            Chipset::I440FX => EzkvmChipset::I440FX,
        }
    }
}

impl From<EzkvmChipset> for Chipset {
    fn from(chipset: EzkvmChipset) -> Self {
        match chipset {
            EzkvmChipset::Q35 => Chipset::Q35(Q35Chipset::new(Arc::new(Mutex::new(
                crate::runtime::BusDeviceRegistry(HashMap::new()),
            )))),
            EzkvmChipset::I440FX => Chipset::I440FX,
        }
    }
}

fn chipset_handler(builder: &mut EzkvmSchemaBuilder, device: &dyn RootDevice) -> Result<(), ()> {
    if let Some(chipset) = device.as_any().downcast_ref::<Chipset>() {
        builder.vm_schema.chipset = Some(chipset.clone().into());
        Ok(())
    } else {
        Err(())
    }
}
