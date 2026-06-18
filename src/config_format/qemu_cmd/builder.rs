use std::{ops::Deref, sync::Arc};

use crate::runtime_model::RuntimeModel;

pub struct QemuArgs(Vec<String>);
#[allow(dead_code)]
impl QemuArgs {
    pub fn new(args: Vec<String>) -> Self {
        QemuArgs(args)
    }

    pub fn extend(&mut self, other: Vec<String>) {
        self.0.extend(other);
    }
}
impl Deref for QemuArgs {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(dead_code)]
pub trait QemuArgsBuilder {
    fn qemu_args(&self) -> QemuArgs;
}

pub struct QemuCommandBuilder {
    runtime_model: Arc<RuntimeModel>,
}
#[allow(dead_code)]
impl QemuCommandBuilder {
    pub fn new(runtime_model: Arc<RuntimeModel>) -> Self {
        Self { runtime_model }
    }

    pub fn build(&self) -> Result<QemuArgs, String> {
        let mut args = QemuArgs(vec![
            "qemu-system-x86_64".to_string(),
            "-name".to_string(),
            self.runtime_model.name().clone(),
        ]);

        // Add CPU and memory arguments
        args.extend(self.runtime_model.cpu().qemu_args());
        args.extend(self.runtime_model.memory().qemu_args());

        // Add chipset arguments
        args.extend(self.runtime_model.chipset().qemu_args());

        // Add device arguments
        //for device in self.runtime_model.devices().iter() {
        //    args.extend(device.qemu_args());
        //}

        // Add boot arguments
        args.extend(self.runtime_model.boot().qemu_args());

        Ok(args)
    }
}
