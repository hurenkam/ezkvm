use crate::resource::lock::{EzkvmError, Lock};
use crate::resource::resource::Resource;
use crate::resource::resource_pool::ResourcePool;
use log::{debug, info};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};

static RESOURCE_MANAGER: Lazy<Arc<Mutex<DataManager>>> =
    Lazy::new(|| Arc::new(Mutex::new(DataManager::new("/etc/ezkvm/resource"))));

#[derive(Debug)]
pub struct DataManager {
    resources: HashMap<String, ResourcePool>,
    locks: HashMap<String, Lock>,
    locked_resources: HashMap<String, String>,
    current_lock: Lock,
}

impl DataManager {
    fn new(path: &str) -> Self {
        let mut resources = load_resource_pools().unwrap_or_default();
        info!("ResourceManager::new() resources:\n{:?}", resources);

        let locks = load_machine_locks().unwrap_or_default();
        info!("ResourceManager::new() locks:\n{:?}", locks);

        let locked_resources = find_locked_resources(&resources, &locks);
        info!(
            "ResourceManager::new() locked_resources:\n{:?}",
            locked_resources
        );

        let result = Self {
            resources,
            locks,
            locked_resources,
            current_lock: Lock::default(),
        };

        //info!("ResourceManager::new():\n{:?}",result);
        result
    }

    pub fn instance() -> Arc<Mutex<DataManager>> {
        RESOURCE_MANAGER.clone()
    }

    pub fn claim_resource(&mut self, pool: String) -> Result<String, EzkvmError> {
        debug!("DataManager::claim_resource('{}')", pool);

        if let Some(resource_pool) = self.resources.get(&pool) {
            let id = resource_pool.claim_resource(&self.locked_resources)?;
            self.current_lock.add_resource(id.clone());
            self.locked_resources
                .insert(id.clone(), self.current_lock.get_name());
            return Ok(id);
        }

        debug!(
            "DataManager::claim_resource() Resource '{}' not available.",
            pool
        );
        Err(EzkvmError::ResourceNotAvailable { pool })
    }

    pub fn get_resource(&self, pool: &String, id: &String) -> Option<&Resource> {
        if let Some(pool) = self.resources.get(pool) {
            pool.get_resource(id)
        } else {
            None
        }
    }
    /*
        pub fn get_current_lock(&mut self) -> &Option<Lock> {
            if self.current_lock.is_none() {
                self.current_lock = Some(Lock::default());
            }

            &self.current_lock
        }
    */
}

fn load_resource_pools() -> Result<HashMap<String, ResourcePool>, EzkvmError> {
    debug!("read_locks()");
    let mut resource_pools = HashMap::from([]);

    let files = fs::read_dir("/etc/ezkvm/resources/").map_err(|_| EzkvmError::OpenError {
        file: "/etc/ezkvm/resources/".to_string(),
    })?;
    for file in files {
        if let Ok(entry) = file {
            if let Ok(file_name) = entry.file_name().into_string() {
                if let Some(base_name) = file_name.strip_suffix(".yaml") {
                    debug!("load_resource_pools(): {:?}", base_name);
                    if let Ok(resource_pool) = ResourcePool::read(base_name) {
                        resource_pools.insert(base_name.to_string(), resource_pool);
                    }
                }
            }
        }
    }

    Ok(resource_pools)
}

fn load_machine_locks() -> Result<HashMap<String, Lock>, EzkvmError> {
    debug!("read_locks()");
    let mut locks = HashMap::from([]);

    let files = fs::read_dir("/var/ezkvm/lock/").map_err(|_| EzkvmError::OpenError {
        file: "/var/ezkvm/lock/".to_string(),
    })?;
    for file in files {
        if let Ok(entry) = file {
            if let Ok(file_name) = entry.file_name().into_string() {
                if let Some(base_name) = file_name.strip_suffix(".yaml") {
                    debug!("LockList::read(): {:?}", base_name);
                    if let Ok(lock) = Lock::read(base_name) {
                        locks.insert(base_name.to_string(), lock);
                    }
                }
            }
        }
    }

    Ok(locks)
}

fn find_locked_resources(
    resources: &HashMap<String, ResourcePool>,
    locks: &HashMap<String, Lock>,
) -> HashMap<String, String> {
    let mut result = HashMap::from([]);

    for (_, lock) in locks {
        for locked_resource in lock.get_resources() {
            result.insert(locked_resource, lock.get_name());
        }
    }

    result
}
