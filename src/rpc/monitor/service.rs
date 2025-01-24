use crate::rpc::connection::ConnectionApi;
use crate::rpc::error::RpcError;
use crate::rpc::monitor::interface::MonitorServiceApi;
use log::{info, trace};
use std::sync::Arc;

pub struct MonitorService<C: ConnectionApi> {
    connection: C,
}

impl<C: ConnectionApi> MonitorServiceApi<C> for MonitorService<C> {
    fn new(connection: C) -> Result<Arc<Self>, RpcError> {
        info!("MonitorService[].new()");

        let data = connection.read_raw()?;
        trace!("{}", data);

        connection.write_raw("{\"execute\":\"qmp_capabilities\"}".to_string())?;
        let data = connection.read_raw()?;
        trace!("{}", data);

        Ok(Arc::new(MonitorService { connection }))
    }
    fn raw(&self, cmd: String) -> Result<String, RpcError> {
        self.connection.write_raw(cmd)?;
        self.connection.read_raw()
    }
}
