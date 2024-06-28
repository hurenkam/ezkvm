use std::fmt::Debug;
use log::trace;
use serde::Deserialize;
use crate::config::config::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait Resource:
    where
        Self: QemuDevice + Sync + Send + Debug
{
    fn get_id(&self) -> String;
    fn get_tags(&self) -> Vec<String>;
}

impl Clone for Box<dyn Resource> {
    fn clone(&self) -> Box<dyn Resource> {
        todo!()
    }
}
