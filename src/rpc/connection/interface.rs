use crate::rpc::error::RpcError;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait ConnectionApi {
    fn write<C: Serialize>(&self, c: C) -> Result<(), RpcError>;

    fn read<D: DeserializeOwned>(&self) -> Result<D, RpcError>;
}
