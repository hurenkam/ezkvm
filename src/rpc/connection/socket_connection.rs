use crate::rpc::connection::interface::ConnectionApi;
use crate::rpc::error::RpcError;
use log::info;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::Mutex;

pub struct SocketConnection {
    stream: Mutex<UnixStream>,
}

#[allow(unused)]
impl SocketConnection {
    pub fn connect(path: &str) -> Result<Self, RpcError> {
        let stream = Mutex::new(UnixStream::connect(path).map_err(|_| RpcError::ConnectionError)?);
        Ok(Self { stream })
    }
}

impl ConnectionApi for SocketConnection {
    fn write<C: Serialize>(&self, c: C) -> Result<(), RpcError> {
        let data = serde_json::to_string(&c).map_err(|_| RpcError::SerializeError)?;
        info!("SocketConnection::write({})", data.clone());
        self.stream
            .lock()
            .unwrap()
            .write_all(data.as_bytes())
            .map_err(|_| RpcError::WriteError)
    }

    fn read<D: DeserializeOwned>(&self) -> Result<D, RpcError> {
        let mut buffer = vec![0; 10240];
        let count = self
            .stream
            .lock()
            .unwrap()
            .read(&mut buffer)
            .map_err(|_| RpcError::ReadError)?;
        let mut data = if count == 0 {
            String::new()
        } else {
            let data = &buffer[..count];
            String::from_utf8_lossy(data).to_string()
        };
        if data.ends_with('\n') {
            data.truncate(data.len() - 1)
        };
        info!("SocketConnection::read({})", data.clone());
        serde_json::from_str(data.as_str()).map_err(|_| RpcError::DeserializeError)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() {}
}
