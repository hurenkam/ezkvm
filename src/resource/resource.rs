use std::fmt::Debug;
use log::trace;
use serde::Deserialize;
use crate::config::config::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait Resource:
    where
        Self: BoxedResourceClone + QemuDevice + Sync + Send + Debug
{
    fn get_id(&self) -> String;
    fn get_tags(&self) -> Vec<String>;
}

impl Clone for Box<dyn Resource> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

pub trait BoxedResourceClone {
    fn boxed_clone(&self) -> Box<dyn Resource>;
}

impl<T> BoxedResourceClone for T
where
    T: 'static + Resource + Clone + Send + Sync
{
    fn boxed_clone(&self) -> Box<dyn Resource> {
        Box::new(self.clone())
    }
}
