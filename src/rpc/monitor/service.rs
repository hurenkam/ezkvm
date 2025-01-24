use crate::rpc::connection::ConnectionApi;
use crate::rpc::error::RpcError;
use std::sync::Arc;
use crate::rpc::monitor::interface::MonitorServiceApi;

pub struct MonitorService<C: ConnectionApi> {
    connection: C,
}

impl<C: ConnectionApi> MonitorServiceApi<C> for MonitorService<C> {
    fn new(connection: C) -> Arc<Self> {
        Arc::new(MonitorService {
            connection,
        })
    }
    fn raw(&self, cmd: String)-> Result<String, RpcError> {
        self.connection.write(cmd)?;
        self.connection.read::<String>()
    }
}
