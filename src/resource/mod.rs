pub mod resource_collection;
pub mod resource_pool;
mod resource;
mod pci_resource;
mod gpu_resource;
mod x550_vf_resource;

pub trait FromFile {
    type Error;

    fn from_file(file_name: &str) -> Result<Self, Self::Error> where Self: Sized;
}
