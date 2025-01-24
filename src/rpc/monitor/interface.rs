use crate::rpc::connection::ConnectionApi;
use crate::rpc::error::RpcError;
use std::sync::Arc;

#[allow(unused)]
pub trait MonitorServiceApi<C: ConnectionApi> {
    fn new(connection: C) -> Result<Arc<Self>, RpcError>;
    fn raw(&self, cmd: String) -> Result<String, RpcError>;
}

pub use crate::rpc::monitor::service::MonitorService;
