use crate::runtime::{RootDevice, RootDeviceKind};

/// ⚠️ RAWARGS: The inner String is verbatim — never split, tokenize, or reorder it.
/// Cross-references between args depend on relative ordering. Access via `.0`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RawArgs(pub String);

impl RootDevice for RawArgs {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "raw_args" }
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::RawArgs }
}
