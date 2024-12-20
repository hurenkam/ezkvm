use crate::resource::resource::Resource;
use log::debug;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use crate::osal::OsalError;

#[derive(Debug, Deserialize)]
pub struct ResourcePool {
    id: String,
    devices: Vec<Resource>,
}

#[allow(dead_code)]
impl ResourcePool {
    pub fn read(name: &str) -> Result<ResourcePool, OsalError> {
        debug!("ResourcePool::read({})", name);

        let mut file =
            File::open(format!("/etc/ezkvm/resources/{}.yaml", name)).expect("Unable to open file");
        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .expect("Unable to read file");

        let resource_pool: ResourcePool =
            serde_yaml::from_str(contents.as_str()).map_err(|_| OsalError::ParseError(Some(name.to_string())))?;
        Ok(resource_pool)
    }

    pub fn get_ids(&self) -> Vec<String> {
        self.devices.iter().map(|device| device.get_id()).collect()
    }

    pub fn claim_resource(
        &self,
        locked_resources: &HashMap<String, String>,
    ) -> Result<String, OsalError> {
        debug!("ResourcePool::claim_resource()");
        for id in self.get_ids() {
            if !locked_resources.contains_key(&id) {
                debug!("claim_resource({})  found: {}", self.id, id);
                return Ok(id);
            }
        }
        debug!(
            "ResourcePool::claim_resource() Resource {} not available.",
            self.id.clone()
        );
        Err(OsalError::Busy(Some(self.id.clone())))
    }

    pub fn get_resource(&self, id: &String) -> Option<&Resource> {
        for resource in &self.devices {
            if resource.get_id() == *id {
                return Some(resource);
            }
        }

        None
    }
}
