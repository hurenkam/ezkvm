use crate::rpc::connection::interface::ConnectionApi;
use crate::rpc::error::RpcError;
use log::info;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Mutex;

pub struct SocketConnection {
    id: String,
    stream: Mutex<BufReader<UnixStream>>,
}

#[allow(unused)]
impl SocketConnection {
    pub fn connect(path: String) -> Result<Self, RpcError> {
        let stream = Mutex::new(BufReader::new(
            UnixStream::connect(path.clone()).map_err(|_| RpcError::ConnectionError)?,
        ));
        Ok(Self { id: path, stream })
    }

    #[cfg(test)]
    fn from_stream(id: String, stream: UnixStream) -> Self {
        Self {
            id,
            stream: Mutex::new(BufReader::new(stream)),
        }
    }
}

impl ConnectionApi for SocketConnection {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn write_raw(&self, data: String) -> Result<(), RpcError> {
        info!(
            "SocketConnection[{}]::write_raw({})",
            self.id(),
            data.clone()
        );

        let mut payload = data;
        if !payload.ends_with('\n') {
            payload.push('\n');
        }

        let mut stream = self.stream.lock().unwrap();
        stream
            .get_mut()
            .write_all(payload.as_bytes())
            .map_err(|_| RpcError::WriteError)
    }

    fn read_raw(&self) -> Result<String, RpcError> {
        let mut data = String::new();
        let count = self
            .stream
            .lock()
            .unwrap()
            .read_line(&mut data)
            .map_err(|_| RpcError::ReadError)?;

        if count == 0 {
            return Err(RpcError::ConnectionError);
        }

        while data.ends_with(['\n', '\r']) {
            data.pop();
        }

        info!(
            "SocketConnection[{}]::read_raw({})",
            self.id(),
            data.clone()
        );

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn read_raw_handles_coalesced_frames() {
        let (left, mut right) = UnixStream::pair().unwrap();
        let conn = SocketConnection::from_stream("pair-left".to_string(), left);

        right
            .write_all(b"{\"execute\":\"a\"}\n{\"execute\":\"b\"}\n")
            .unwrap();

        assert_eq!(conn.read_raw().unwrap(), "{\"execute\":\"a\"}");
        assert_eq!(conn.read_raw().unwrap(), "{\"execute\":\"b\"}");
    }

    #[test]
    fn read_raw_handles_split_frame() {
        let (left, mut right) = UnixStream::pair().unwrap();
        let conn = SocketConnection::from_stream("pair-left".to_string(), left);

        let writer = thread::spawn(move || {
            right.write_all(b"{\"execute\":").unwrap();
            thread::sleep(Duration::from_millis(10));
            right.write_all(b"\"ping\"}\n").unwrap();
        });

        assert_eq!(conn.read_raw().unwrap(), "{\"execute\":\"ping\"}");
        writer.join().unwrap();
    }

    #[test]
    fn write_raw_appends_newline() {
        let (left, mut right) = UnixStream::pair().unwrap();
        right
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let conn = SocketConnection::from_stream("pair-left".to_string(), left);

        conn.write_raw("{\"execute\":\"ping\"}".to_string())
            .unwrap();

        let mut buffer = [0u8; 64];
        let count = right.read(&mut buffer).unwrap();
        let received = String::from_utf8_lossy(&buffer[..count]).to_string();

        assert_eq!(received, "{\"execute\":\"ping\"}\n");
    }
}
