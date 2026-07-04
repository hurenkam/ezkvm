use std::{
    io::{BufRead, BufReader, ErrorKind, Write},
    os::unix::net::UnixStream,
    time::Duration,
};

use serde_json::{Value, json};

const QMP_IO_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_QMP_MESSAGES_PER_COMMAND: usize = 32;

pub(super) fn execute_qmp_command(
    socket_path: &str,
    command: &str,
    tolerate_disconnect: bool,
) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|e| format!("failed to connect to QMP socket '{socket_path}': {e}"))?;
    stream
        .set_read_timeout(Some(QMP_IO_TIMEOUT))
        .map_err(|e| format!("failed to set QMP read timeout: {e}"))?;
    stream
        .set_write_timeout(Some(QMP_IO_TIMEOUT))
        .map_err(|e| format!("failed to set QMP write timeout: {e}"))?;

    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|e| format!("failed to clone QMP socket stream: {e}"))?,
    );

    let greeting = read_json_message(&mut reader)?;
    if greeting
        .as_ref()
        .and_then(|message| message.get("QMP"))
        .is_none()
    {
        return Err("invalid QMP greeting: missing 'QMP' field".to_string());
    }

    send_json_message(&mut stream, &json!({ "execute": "qmp_capabilities" }))?;
    wait_for_command_result(&mut reader, "qmp_capabilities", false)?;

    send_json_message(&mut stream, &json!({ "execute": command }))?;
    wait_for_command_result(&mut reader, command, tolerate_disconnect)
}

fn send_json_message(stream: &mut UnixStream, message: &Value) -> Result<(), String> {
    let wire = serde_json::to_string(message)
        .map_err(|e| format!("failed to serialize QMP command: {e}"))?;
    stream
        .write_all(wire.as_bytes())
        .map_err(|e| format!("failed to write QMP command: {e}"))?;
    stream
        .write_all(b"\n")
        .map_err(|e| format!("failed to terminate QMP command: {e}"))
}

fn read_json_message(reader: &mut BufReader<UnixStream>) -> Result<Option<Value>, String> {
    let mut line = String::new();

    match reader.read_line(&mut line) {
        Ok(0) => Ok(None),
        Ok(_) => {
            let payload = line.trim();
            if payload.is_empty() {
                return read_json_message(reader);
            }
            serde_json::from_str(payload)
                .map(Some)
                .map_err(|e| format!("failed to parse QMP response '{payload}': {e}"))
        }
        Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
            Err("timed out while waiting for QMP response".to_string())
        }
        Err(error) => Err(format!("failed reading from QMP socket: {error}")),
    }
}

fn wait_for_command_result(
    reader: &mut BufReader<UnixStream>,
    command: &str,
    tolerate_disconnect: bool,
) -> Result<(), String> {
    for _ in 0..MAX_QMP_MESSAGES_PER_COMMAND {
        let Some(message) = read_json_message(reader)? else {
            return if tolerate_disconnect {
                Ok(())
            } else {
                Err(format!(
                    "QMP socket closed before '{command}' returned a result"
                ))
            };
        };

        if message.get("return").is_some() {
            return Ok(());
        }

        if let Some(error) = message.get("error") {
            return Err(format!("QMP command '{command}' failed: {error}"));
        }

        if message.get("event").is_some() {
            continue;
        }
    }

    Err(format!(
        "QMP command '{command}' produced no result after {MAX_QMP_MESSAGES_PER_COMMAND} messages"
    ))
}
