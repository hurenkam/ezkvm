use std::fmt::Debug;

#[typetag::deserialize(tag = "type")]
pub trait StoragePayload: Debug {
    fn get_drive_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
    fn get_device_options(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}
