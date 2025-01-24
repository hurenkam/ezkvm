use crate::rpc::error::RpcError;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait ConnectionApi {
    fn write<C: Serialize>(&self, c: C) -> Result<(), RpcError> {
        let data = serde_json::to_string(&c).map_err(|_| RpcError::SerializeError)?;
        self.write_raw(data)
    }
    fn read<D: DeserializeOwned>(&self) -> Result<D, RpcError> {
        let data = self.read_raw()?;
        serde_json::from_str(data.as_str()).map_err(|_| RpcError::DeserializeError)
    }

    fn write_raw(&self, s: String) -> Result<(), RpcError>;
    fn read_raw(&self) -> Result<String, RpcError>;
}
