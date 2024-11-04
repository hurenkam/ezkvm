use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NetworkHeader {}

impl NetworkHeader {
    pub fn get_netdev_options(&self) -> Vec<String> {
        vec![]
    }

    pub fn get_device_options(&self) -> Vec<String> {
        vec![]
    }
}
