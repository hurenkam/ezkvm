//! QMP SHUTDOWN event monitor
//!
//! Spawns a background thread that connects to a QEMU QMP socket, waits for
//! the SHUTDOWN event (emitted when the guest OS initiates a power-off), and
//! then sends the `quit` command so the QEMU process exits cleanly.
//!
//! This mirrors what Proxmox's `qmeventd` does, preventing QEMU from
//! spinning at 100% CPU after a guest shutdown when GPU passthrough teardown
//! hangs.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// Spawn the QMP shutdown monitor thread.
///
/// The thread retries connecting to `socket_path` until QEMU is ready,
/// then monitors QMP events. On a `SHUTDOWN` event it sends `quit` and exits.
/// The thread silently exits if the socket closes for any reason (QEMU already
/// quit, or an error occurred).
pub fn spawn_shutdown_monitor(socket_path: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        if let Err(e) = run_monitor(&socket_path) {
            // Only log unexpected errors, not normal socket-closed cases.
            if !is_connection_closed(&e) {
                eprintln!("[shutdown-monitor] error: {}", e);
            }
        }
    })
}

fn run_monitor(socket_path: &str) -> Result<(), MonitorError> {
    wait_for_socket(socket_path, 60)?;
    let stream = connect_with_retry(socket_path, 20)?;
    let reader_stream = stream.try_clone().map_err(MonitorError::Io)?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    negotiate_capabilities(&mut reader, &mut writer)?;
    monitor_events(&mut reader, &mut writer)
}

fn wait_for_socket(socket_path: &str, max_half_seconds: u32) -> Result<(), MonitorError> {
    for _ in 0..max_half_seconds {
        if std::path::Path::new(socket_path).exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    // One final check before giving up.
    if std::path::Path::new(socket_path).exists() {
        Ok(())
    } else {
        Err(MonitorError::SocketNotFound)
    }
}

fn connect_with_retry(socket_path: &str, retries: u32) -> Result<UnixStream, MonitorError> {
    let mut last_err = std::io::Error::other("no attempts made");
    for _ in 0..retries {
        match UnixStream::connect(socket_path) {
            Ok(s) => return Ok(s),
            Err(e) => {
                last_err = e;
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
    Err(MonitorError::Io(last_err))
}

fn negotiate_capabilities(
    reader: &mut BufReader<UnixStream>,
    writer: &mut UnixStream,
) -> Result<(), MonitorError> {
    // Read QMP greeting {"QMP": {...}}
    let greeting = read_json(reader)?;
    if greeting.get("QMP").is_none() {
        return Err(MonitorError::Protocol("missing QMP greeting".into()));
    }

    // Send qmp_capabilities to exit negotiation mode.
    write_command(writer, r#"{"execute":"qmp_capabilities"}"#)?;

    // Read the {"return": {}} response.
    let _ = read_json(reader)?;

    Ok(())
}

fn monitor_events(
    reader: &mut BufReader<UnixStream>,
    writer: &mut UnixStream,
) -> Result<(), MonitorError> {
    loop {
        let msg = read_json(reader)?;
        if msg
            .get("event")
            .and_then(|e| e.as_str())
            .is_some_and(|e| e == "SHUTDOWN")
        {
            // Guest has initiated shutdown. Send quit so QEMU exits cleanly
            // instead of spinning while waiting for guest teardown to complete.
            let _ = write_command(writer, r#"{"execute":"quit"}"#);
            return Ok(());
        }
    }
}

fn write_command(writer: &mut UnixStream, cmd: &str) -> Result<(), MonitorError> {
    writer
        .write_all(format!("{}\n", cmd).as_bytes())
        .map_err(MonitorError::Io)?;
    writer.flush().map_err(MonitorError::Io)
}

fn read_json(reader: &mut BufReader<UnixStream>) -> Result<serde_json::Value, MonitorError> {
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(MonitorError::Io)?;
        if bytes == 0 {
            return Err(MonitorError::SocketClosed);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return serde_json::from_str(trimmed).map_err(|e| MonitorError::Json(e.to_string()));
    }
}

fn is_connection_closed(e: &MonitorError) -> bool {
    matches!(e, MonitorError::SocketClosed | MonitorError::SocketNotFound)
}

#[derive(Debug)]
enum MonitorError {
    SocketNotFound,
    SocketClosed,
    Io(std::io::Error),
    Protocol(String),
    Json(String),
}

impl std::fmt::Display for MonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorError::SocketNotFound => write!(f, "QMP socket did not appear"),
            MonitorError::SocketClosed => write!(f, "QMP socket closed"),
            MonitorError::Io(e) => write!(f, "IO error: {}", e),
            MonitorError::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            MonitorError::Json(msg) => write!(f, "JSON error: {}", msg),
        }
    }
}
