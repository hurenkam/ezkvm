use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::network::NetworkDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct NetworkPoolDevice {
    pool: String,
    mac: String
}

#[typetag::deserialize(name = "pool")]
impl NetworkDevice for NetworkPoolDevice { }

impl QemuDevice for NetworkPoolDevice {
    fn get_args(&self, index: usize) -> Vec<String> {
        /*
                let pool = self.pool.clone().unwrap_or_default();
                let resource_id = DataManager::instance().lock().unwrap().claim_resource(pool.clone());

                match resource_id {
                    Ok(resource_id) => {
                        if let Some(resource) = DataManager::instance().lock().unwrap().get_resource(&pool,&resource_id).clone() {
                            resource.get_qemu_args(0)
                        } else {
                            vec![]
                        }
                    }
                    Err(_) => {
                        panic!("Unable to claim resource from pool '{}'",pool);
                    }
                }
         */
        vec![]
    }
}
