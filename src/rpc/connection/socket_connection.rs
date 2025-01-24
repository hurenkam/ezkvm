use crate::rpc::connection::interface::ConnectionApi;
use crate::rpc::error::RpcError;
use log::info;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::Mutex;

pub struct SocketConnection {
    stream: Mutex<UnixStream>,
}

#[allow(unused)]
impl SocketConnection {
    pub fn connect(path: String) -> Result<Self, RpcError> {
        let stream = Mutex::new(UnixStream::connect(path).map_err(|_| RpcError::ConnectionError)?);
        Ok(Self { stream })
    }
}

impl ConnectionApi for SocketConnection {
    fn write_raw(&self, data: String) -> Result<(), RpcError> {
        info!("SocketConnection::write_raw({})", data.clone());
        self.stream
            .lock()
            .unwrap()
            .write_all(data.as_bytes())
            .map_err(|_| RpcError::WriteError)
    }

    fn read_raw(&self) -> Result<String, RpcError> {
        let mut buffer = vec![0; 65536];
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

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() {}
}
