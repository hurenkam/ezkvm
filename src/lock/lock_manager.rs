use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use log::{debug, trace};
use once_cell::sync::Lazy;
use serde::Deserialize;
use crate::lock::lock::Lock;
use crate::resource::FromFile;
use crate::resource::resource_pool::ResourcePool;
use crate::types::EzkvmError;

const LOCK_DIRECTORY: &str = "var";
static LOCK_MANAGER: Lazy<Arc<LockManager>> = Lazy::new(||Arc::new(LockManager::new()));

#[derive(Debug)]
pub struct LockManager {
    locks: Mutex<HashMap<String,Arc<Mutex<Lock>>>>
}

impl LockManager {
    fn new() -> Self {
        Self {
            locks: Mutex::new(get_locks(LOCK_DIRECTORY)
                .unwrap_or_else(|_| HashMap::default()))
        }
    }

    pub fn instance() -> Arc<Self> {
        debug!("ResourceCollection::instance()");
        LOCK_MANAGER.clone()
    }

    pub fn get_locked_resources(&self) -> Vec<String> {
        let mut result = vec![];
        if let Ok(locks) = self.locks.lock() {
            for (key,lock) in locks.iter() {
                if let Ok(lock) = lock.lock() {
                    result.extend(lock.resources.clone());
                }
            }
        }
        result
    }

    pub fn get_lock(&self, id: String) -> Option<Arc<Mutex<Lock>>> {
        debug!("ResourceCollection.get_resource()");
        if let Ok(locks) = self.locks.lock() {
            if let Some(lock) = locks.get(&id) {
                return Some(lock.clone())
            }
        }
        None
    }

    pub fn create_lock(&self, id: String) -> Option<Arc<Mutex<Lock>>> {
        debug!("ResourceCollection.create_lock()");
        if let Ok(mut locks) = self.locks.lock() {
            if locks.get(&id).is_none() {

                let filename = format!("{}/{}.yaml", LOCK_DIRECTORY, id);
                let pid = 0;
                let lock = Arc::new(Mutex::new(Lock::new(filename, pid, vec![])));
                locks.insert(id,lock.clone());

                return Some(lock.clone());
            }
        }

        None
    }

    pub fn delete_lock(&self, id: String) {
        debug!("ResourceCollection.get_resource()");
        if let Ok(mut locks) = self.locks.lock() {
            if let Some(lock) = locks.get(&id) {
                if let Ok(mut lock) = lock.lock() {
                    let _ = lock.delete();
                } else { return }
            } else { return }
            locks.remove(&id);
        }
    }
}

fn get_locks(directory: &str) -> Result<HashMap<String,Arc<Mutex<Lock>>>,EzkvmError> {
    debug!("get_locks({})", directory);
    let mut locks = HashMap::from([]);
    let files = fs::read_dir(directory).map_err(|_|EzkvmError::OpenError { file: directory.to_string() })?;

    for file in files {
        if let Ok(entry) = file {
            if let Ok(file_name) = entry.file_name().into_string() {
                if let Some(base_name) = file_name.strip_suffix(".yaml") {
                    let path = format!("{}/{}.yaml",directory,base_name);
                    trace!("lock_manager::get_locks(): found file {}",path);
                    if let Ok(lock) = Lock::read(path.as_str()) {
                        locks.insert(base_name.to_string(), Arc::new(Mutex::new(lock)));
                    }
                }
            }
        }
    }

    Ok(locks)
}
