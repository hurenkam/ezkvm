use std::fmt::Debug;

#[typetag::deserialize(tag = "type")]
pub trait NetworkPayload: Debug {
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
    fn get_device_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}
