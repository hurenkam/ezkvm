use crate::rpc::connection::ConnectionApi;
use crate::rpc::error::RpcError;
use derive_getters::Getters;
use derive_new::new;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Getters, new)]
pub struct GuestAgentInfo {
    version: String,
    supported_commands: Vec<GuestAgentSupportedCommand>,
}

#[derive(Clone, Debug, PartialEq, Getters, new)]
pub struct GuestAgentSupportedCommand {
    enabled: bool,
    name: String,
    success_response: bool,
}

#[allow(unused)]
pub trait GuestAgentServiceApi<C: ConnectionApi> {
    fn new(connection: C) -> Arc<Self>;
    fn sync(&self) -> Result<(), RpcError>;
    fn info(&self) -> Result<GuestAgentInfo, RpcError>;
    fn shutdown(&self) -> Result<(), RpcError>;
    fn hibernate(&self) -> Result<(), RpcError>;
    fn raw(&self, cmd: String)-> Result<String, RpcError>;
}

pub use crate::rpc::agent::service::GuestAgentService;