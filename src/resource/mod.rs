pub mod resource_collection;
pub mod resource_pool;

pub trait FromFile {
    type Error;

    fn from_file(file_name: &str) -> Result<Self, Self::Error> where Self: Sized;
}
