use std::{
    io::{BufRead, BufReader, ErrorKind, Write},
    os::unix::net::UnixStream,
};

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QmpError {
    #[error("failed to connect to QMP socket at {path}: {source}")]
    Connect {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("QMP socket closed before greeting was received")]
    NoGreeting,
    #[error("qmp_capabilities negotiation failed: {desc}")]
    CapabilitiesFailed { desc: String },
    #[error("QMP command '{command}' returned an error: {desc}")]
    CommandFailed { command: String, desc: String },
    #[error("I/O error communicating with QMP socket: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse QMP JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct QmpClient {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl QmpClient {
    pub fn connect(socket_path: &str) -> Result<Self, QmpError> {
        let stream = UnixStream::connect(socket_path).map_err(|source| QmpError::Connect {
            path: socket_path.to_string(),
            source,
        })?;
        let reader = BufReader::new(stream.try_clone()?);
        let mut client = Self { stream, reader };

        let mut line = String::new();
        if client.reader.read_line(&mut line)? == 0 {
            return Err(QmpError::NoGreeting);
        }

        let response = client.execute("qmp_capabilities", None)?;
        if let Some(err) = response.get("error") {
            return Err(QmpError::CapabilitiesFailed {
                desc: err
                    .get("desc")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            });
        }

        Ok(client)
    }

    pub fn execute(&mut self, command: &str, args: Option<Value>) -> Result<Value, QmpError> {
        let mut request = serde_json::json!({ "execute": command });
        if let Some(arguments) = args {
            request["arguments"] = arguments;
        }

        let mut payload = serde_json::to_string(&request)?;
        payload.push('\n');
        self.stream.write_all(payload.as_bytes())?;

        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Err(QmpError::Io(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                format!("QMP socket closed while awaiting '{command}' response"),
            )));
        }

        Ok(serde_json::from_str(&line)?)
    }

    pub fn system_powerdown(&mut self) -> Result<(), QmpError> {
        let response = self.execute("system_powerdown", None)?;
        if let Some(err) = response.get("error") {
            return Err(QmpError::CommandFailed {
                command: "system_powerdown".to_string(),
                desc: err
                    .get("desc")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            });
        }
        Ok(())
    }

    pub fn quit(&mut self) -> Result<(), QmpError> {
        match self.execute("quit", None) {
            Ok(response) => {
                if let Some(err) = response.get("error") {
                    return Err(QmpError::CommandFailed {
                        command: "quit".to_string(),
                        desc: err
                            .get("desc")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                    });
                }
                Ok(())
            }
            Err(QmpError::Io(_)) => Ok(()),
            Err(other) => Err(other),
        }
    }

    pub fn system_reset_fire_and_forget(&mut self) -> Result<(), QmpError> {
        let request = serde_json::json!({ "execute": "system_reset" });
        let mut payload = serde_json::to_string(&request)?;
        payload.push('\n');
        self.stream.write_all(payload.as_bytes())?;
        Ok(())
    }
}
