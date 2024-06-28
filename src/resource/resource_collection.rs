use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use log::{debug, trace};
use once_cell::sync::Lazy;
use serde::Deserialize;
use crate::resource::FromFile;
use crate::resource::resource::Resource;
use crate::resource::resource_pool::ResourcePool;
use crate::types::EzkvmError;

pub const RESOURCE_DIRECTORY: &str = "etc/resource";
static RESOURCE_MANAGER: Lazy<Arc<ResourceCollection>> = Lazy::new(||Arc::new(ResourceCollection::new()));

#[derive(Debug)]
pub struct ResourceCollection {
    resource_pools: HashMap<String,ResourcePool>
}

impl ResourceCollection {
    fn new() -> Self {
        Self {
            resource_pools: get_resource_pools(RESOURCE_DIRECTORY)
                .unwrap_or_else(|_| HashMap::default())
        }
    }

    pub fn instance() -> Arc<Self> {
        debug!("ResourceCollection::instance()");
        RESOURCE_MANAGER.clone()
    }

    pub fn get_pool_ids(&self) -> Vec<String> {
        debug!("ResourceCollection.get_pool_ids()");
        self.resource_pools.keys().map(|v|v.clone()).collect()
    }

    pub fn get_resource_ids(&self, pool: String) -> Vec<String> {
        debug!("ResourceCollection.get_resource_ids()");
        if let Some(pool) = self.resource_pools.get(&pool) {
            pool.get_resource_ids()
        } else {
            vec![]
        }
    }

    pub fn get_resource(&self, pool: String, id: String) -> Option<Box<dyn Resource>> {
        debug!("ResourceCollection.get_resource()");
        if let Some(pool) = self.resource_pools.get(&pool) {
            pool.get_resource(id)
        } else {
            None
        }
    }
}

fn get_resource_pools(directory: &str) -> Result<HashMap<String,ResourcePool>,EzkvmError> {
    debug!("get_resource_pools({})", directory);
    let mut resource_pools = HashMap::from([]);
    let files = fs::read_dir(directory).map_err(|_|EzkvmError::OpenError { file: directory.to_string() })?;

    for file in files {
        if let Ok(entry) = file {
            if let Ok(file_name) = entry.file_name().into_string() {
                if let Some(base_name) = file_name.strip_suffix(".yaml") {
                    debug!("load_resource_pools(): {:?}", base_name);
                    if let Ok(resource_pool) = ResourcePool::from_file(base_name) {
                        resource_pools.insert(base_name.to_string(), resource_pool);
                    }
                }
            }
        }
    }

    Ok(resource_pools)
}
