use crate::rpc::agent::commands::{GuestHibernateRequest, GuestInfoRequest, GuestInfoResponse, GuestShutdownRequest, GuestSyncRequest, GuestSyncResponse};
use crate::rpc::agent::{GuestAgentInfo, GuestAgentServiceApi};
use crate::rpc::connection::ConnectionApi;
use crate::rpc::error::RpcError;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

pub struct GuestAgentService<C: ConnectionApi> {
    connection: C,
    id: AtomicU32,
}

impl<C: ConnectionApi> GuestAgentServiceApi<C> for GuestAgentService<C> {
    fn new(connection: C) -> Arc<Self> {
        Arc::new(GuestAgentService {
            connection,
            id: AtomicU32::new(0),
        })
    }

    fn sync(&self) -> Result<(), RpcError> {
        let id = self.id.fetch_add(1, Relaxed);

        self.connection.write(GuestSyncRequest::new(id))?;
        let response = self.connection.read::<GuestSyncResponse>()?;

        if response.result == id {
            return Ok(());
        }

        Err(RpcError::SyncError)
    }

    fn info(&self) -> Result<GuestAgentInfo, RpcError> {
        self.connection.write(GuestInfoRequest::new())?;
        Ok(self.connection.read::<GuestInfoResponse>()?.into())
    }

    fn shutdown(&self) -> Result<(), RpcError> {
        self.connection
            .write(GuestShutdownRequest::new("powerdown".to_string()))
    }

    fn hibernate(&self) -> Result<(), RpcError> {
        self.connection
            .write(GuestHibernateRequest::new())
    }

    fn raw(&self, cmd: String)-> Result<String, RpcError> {
        self.connection.write(cmd)?;
        self.connection.read::<String>()
    }
}
/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::connection::SocketConnection;

    #[test_log::test]
    fn test_sync_ok() {
        let connection = SocketConnection::connect("/var/ezkvm/gyndine.qga");
        assert!(connection.is_ok());
        let agent = GuestAgentService::new(connection.unwrap());

        let result = agent.sync();
        assert!(result.is_ok());
    }

    #[test_log::test]
    fn test_info_ok() {
        let connection = SocketConnection::connect("/var/ezkvm/gyndine.qga");
        assert!(connection.is_ok());
        let agent = GuestAgentService::new(connection.unwrap());

        let result = agent.info();
        assert!(result.is_ok());
    }

    #[test_log::test]
    fn test_shutdown() {
        let connection = SocketConnection::connect("/var/ezkvm/gyndine.qga");
        assert!(connection.is_ok());
        let agent = GuestAgentService::new(connection.unwrap());

        let result = agent.shutdown();
        assert!(result.is_ok());
    }
}
*/
