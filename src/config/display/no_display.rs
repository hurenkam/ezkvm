use crate::config::display::Display;
use crate::config::types::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct NoDisplay {}
impl NoDisplay {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self {})
    }
}

impl QemuDevice for NoDisplay {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}

#[typetag::deserialize(name = "no_display")]
impl Display for NoDisplay {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let display = NoDisplay {};
        let expected: Vec<String> = vec![];
        assert_eq!(display.get_qemu_args(0), expected);
    }
}
