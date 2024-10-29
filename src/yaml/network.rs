use crate::resource::data_manager::DataManager;
//use crate::yaml::QemuDevice;
use crate::config::QemuDevice;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Network {
    controller: String,
    driver: String,
    pool: Option<String>,
    mac: String,
}
impl QemuDevice for Network {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        match self.controller.as_str() {
            "pool" => {
                let pool = self.pool.clone().unwrap_or_default();
                let resource_id = DataManager::instance()
                    .lock()
                    .unwrap()
                    .claim_resource(pool.clone());

                match resource_id {
                    Ok(resource_id) => {
                        if let Some(resource) = DataManager::instance()
                            .lock()
                            .unwrap()
                            .get_resource(&pool, &resource_id)
                        {
                            resource.get_qemu_args(0)
                        } else {
                            vec![]
                        }
                    }
                    Err(_) => {
                        panic!("Unable to claim resource from pool '{}'", pool);
                    }
                }
            }
            _ => {
                vec![
                    //format!("-netdev id=hostnet{},type={},script=/etc/ezkvm/qemu-ifup.sh,downscript=no",index,self.controller),
                    format!(
                        "-netdev id=hostnet{},type={},br=vmbr0",
                        index, self.controller
                    ),
                    format!(
                        "-device id=net{},driver={},netdev=hostnet{},mac={},bus=pci.1,addr=0x0",
                        index, self.driver, index, self.mac
                    ),
                ]
            }
        }
    }
}
