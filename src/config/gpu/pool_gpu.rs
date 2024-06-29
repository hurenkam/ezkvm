use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::gpu::Gpu;
use crate::resource::resource_collection::ResourceCollection;

#[derive(Deserialize,Debug,Clone)]
pub struct PoolGpu {
    pool_id: String,
    resource_id: Option<String>
}

impl QemuDevice for PoolGpu {
    fn get_args(&self, index: usize) -> Vec<String> {
        let pool_id = self.pool_id.clone();
        let resource_id = match &self.resource_id {
            Some(resource_id) => resource_id.clone(),
            _ => todo!()
        };

        let resource_collection = ResourceCollection::instance();
        if let Some(resource) = resource_collection.get_resource(pool_id, resource_id) {
            resource.get_args(index)
        } else {
            vec![]
        }
    }
}

#[typetag::deserialize(name = "pool")]
impl Gpu for PoolGpu {}
