use std::fs::File;
use std::io::Read;
use log::debug;
use serde::Deserialize;
use crate::resource::FromFile;
use crate::resource::resource::Resource;
use crate::resource::resource_collection::RESOURCE_DIRECTORY;
use crate::types::EzkvmError;

#[derive(Debug,Deserialize)]
pub struct ResourcePool {
    id: String,
    devices: Vec<Box<dyn Resource>>
}
impl ResourcePool {
    pub fn get_resource_ids(&self) -> Vec<String> {
        debug!("ResourcePool.get_resource_ids()");
        let mut result = vec![];

        for resource in &self.devices {
            result.push(resource.get_id())
        }

        result
    }

    pub fn get_resource(&self, id: String) -> Option<Box<dyn Resource>> {
        debug!("ResourcePool.get_resource_ids()");

        for resource in &self.devices {
            if resource.get_id() == id {
                return Some(resource.clone());
            }
        }

        None
    }
}
impl FromFile for ResourcePool {
    type Error = EzkvmError;

    fn from_file(file_name: &str) -> Result<Self, Self::Error> {
        debug!("ResourcePool::from_file({})",file_name);

        let mut file = File::open(format!("{}/{}.yaml", RESOURCE_DIRECTORY, file_name)).expect("Unable to open file");
        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .expect("Unable to read file");

        let resource_pool: ResourcePool = serde_yaml::from_str(contents.as_str()).map_err(|_|EzkvmError::ParseError { file: file_name.to_string() })?;
        Ok(resource_pool)
    }
}
