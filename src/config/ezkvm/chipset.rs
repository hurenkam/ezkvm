use std::{any::TypeId, collections::HashMap};

use crate::{
    config::{EzkvmDeviceHandler, EzkvmSchemaBuilder},
    runtime::RootDevice,
};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct Q35Chipset {
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct I440FXChipset {
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Chipset {
    Q35 { q35: Q35Chipset },
    I440FX { i440fx: I440FXChipset },
}

pub struct EzkvmChipsetHandler;
impl EzkvmDeviceHandler for EzkvmChipsetHandler {
    fn handlers() -> HashMap<TypeId, fn(&mut EzkvmSchemaBuilder, &dyn RootDevice) -> Result<(), ()>>
    {
        HashMap::from([(
            TypeId::of::<crate::runtime::Chipset>(),
            chipset_handler as fn(&mut EzkvmSchemaBuilder, &dyn RootDevice) -> Result<(), ()>,
        )])
    }
}

fn chipset_handler(builder: &mut EzkvmSchemaBuilder, device: &dyn RootDevice) -> Result<(), ()> {
    if let Some(chipset) = device.as_any().downcast_ref::<crate::runtime::Chipset>() {
        builder.with_chipset(chipset.clone().into());
        Ok(())
    } else {
        Err(())
    }
}

impl From<crate::runtime::Chipset> for Chipset {
    fn from(chipset: crate::runtime::Chipset) -> Self {
        match chipset {
            crate::runtime::Chipset::Q35(q35) => Chipset::Q35 {
                q35: Q35Chipset { version: None },
            },
            crate::runtime::Chipset::I440FX => Chipset::I440FX {
                i440fx: I440FXChipset { version: None },
            },
        }
    }
}
/*
impl From<Chipset> for crate::runtime::Chipset {
    fn from(chipset: Chipset) -> Self {
        match chipset {
            Chipset::Q35 { q35: _ } => crate::runtime::Chipset::Q35(crate::runtime::Q35Chipset::new(_)),
            Chipset::I440FX { i440fx: _ } => crate::runtime::Chipset::I440FX,
        }
    }
}
*/