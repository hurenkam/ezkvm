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
    fn device_kind(&self) -> crate::runtime::RootDeviceKind {
        crate::runtime::RootDeviceKind::Chipset
    }
}

impl std::fmt::Display for Chipset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chipset::Q35(q35_chipset) => write!(f, "{}", q35_chipset),
            Chipset::I440FX => write!(f, "I440FX"),
        }
    }
}
