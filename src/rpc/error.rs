#[derive(Clone, Debug)]
#[allow(unused, clippy::enum_variant_names)]
pub enum RpcError {
    ConnectionError,
    SerializeError,
    DeserializeError,
    WriteError,
    ReadError,
    SyncError,
}
