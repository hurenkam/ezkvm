use derive_getters::Getters;
use derive_new::new;

use crate::runtime::RootDevice;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Default, Getters, new)]
pub struct Memory {
    size: usize,
}

impl From<String> for Memory {
    fn from(value: String) -> Self {
        let size = value.parse::<usize>().unwrap();
        Memory { size }
    }
}

impl RootDevice for Memory {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_name(&self) -> &str {
        "memory"
    }
}
