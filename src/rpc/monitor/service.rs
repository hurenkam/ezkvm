use crate::rpc::connection::ConnectionApi;
use crate::rpc::error::RpcError;
use crate::rpc::monitor::commands::{QmpCapabilitiesRequest, QmpCapabilitiesResponse};
use crate::rpc::monitor::interface::MonitorServiceApi;
use log::{info, trace};
use std::sync::Arc;

pub struct MonitorService<C: ConnectionApi> {
    connection: C,
}
impl<C: ConnectionApi> MonitorService<C> {
    fn receive_greeting(&self) -> Result<(), RpcError> {
        info!(
            "MonitorService[{}].receive_greeting()",
            self.connection.id()
        );

        let data = self.connection.read_raw()?;
        trace!("{}", data);

        Ok(())
    }
    fn qmp_capabilities(&self) -> Result<(), RpcError> {
        info!(
            "MonitorService[{}].qmp_capabilities()",
            self.connection.id()
        );

        self.connection.write(QmpCapabilitiesRequest::new())?;
        let _result = self.connection.read::<QmpCapabilitiesResponse>()?;

        Ok(())
    }
}
impl<C: ConnectionApi> MonitorServiceApi<C> for MonitorService<C> {
    fn new(connection: C) -> Result<Arc<Self>, RpcError> {
        info!("MonitorService::new({})", connection.id());

        let monitor = MonitorService { connection };
        monitor.receive_greeting()?;
        monitor.qmp_capabilities()?;

        Ok(Arc::new(monitor))
    }
    fn raw(&self, cmd: String) -> Result<String, RpcError> {
        info!("MonitorService[{}].raw()", self.connection.id());
        self.connection.write_raw(cmd)?;
        self.connection.read_raw()
    }
}
