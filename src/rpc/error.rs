#[derive(Clone, Debug)]
#[allow(unused)]
pub enum RpcError {
    ConnectionError,
    SerializeError,
    DeserializeError,
    WriteError,
    ReadError,
    SyncError,
}
