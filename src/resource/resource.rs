use std::fmt::Debug;
use log::trace;
use serde::Deserialize;

#[typetag::deserialize(tag = "type")]
pub trait Resource: where Self: Sync + Send + Debug {
    fn get_id(&self) -> String;
    fn get_tags(&self) -> Vec<String>;
    fn get_args(&self) -> Vec<String>;
}

impl Clone for Box<dyn Resource> {
    fn clone(&self) -> Box<dyn Resource> {
        todo!()
    }
}
