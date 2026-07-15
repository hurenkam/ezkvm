use crate::runtime::{Q35Chipset, RootDevice};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Chipset {
    Q35(Q35Chipset),
    I440FX,
}

impl RootDevice for Chipset {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_name(&self) -> &str {
        "chipset"
    }
}
