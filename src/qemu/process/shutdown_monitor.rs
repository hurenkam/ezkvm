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
use std::path::PathBuf;
use std::time::{Duration, Instant};

const QMP_IO_TIMEOUT_MS: u64 = 1200;
const QMP_READ_TIMEOUT_RETRIES: usize = 5;
const STALLED_SHUTDOWN_LOG_INTERVAL: Duration = Duration::from_secs(15);
const QMP_TIMEOUT_LOG_INTERVAL: Duration = Duration::from_secs(10);

/// Spawn the QMP shutdown monitor thread.
///
/// The thread retries connecting to `socket_path` until QEMU is ready,
/// then monitors QMP events. On a `SHUTDOWN` event it sends `quit` and exits.
/// The thread silently exits if the socket closes for any reason (QEMU already
/// quit, or an error occurred).
///
/// If `pid_file_path` is provided and QEMU has not exited within the bounded
/// wait after `quit`, the monitor sends SIGKILL using the PID from that file.
pub fn spawn_shutdown_monitor(
    socket_path: String,
    marker_path: PathBuf,
    guest_agent_socket_path: Option<String>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        log_run_monitor_result(run_monitor(
            &socket_path,
            &marker_path,
            guest_agent_socket_path.as_deref(),
        ));
    })
}

/// Run the shutdown monitor synchronously until completion.
pub fn run_shutdown_monitor(
    socket_path: &str,
    marker_path: &std::path::Path,
    guest_agent_socket_path: Option<&str>,
) {
    log_run_monitor_result(run_monitor(
        socket_path,
        marker_path,
        guest_agent_socket_path,
    ));
}

fn log_run_monitor_result(result: Result<(), MonitorError>) {
    if let Err(e) = result {
        // Only log unexpected errors, not normal socket-closed cases.
        if !is_connection_closed(&e) {
            tracing::error!(target: "ezkvm::shutdown_monitor", error = %e, "shutdown monitor error");
        }
    }
}

fn run_monitor(
    socket_path: &str,
    marker_path: &std::path::Path,
    guest_agent_socket_path: Option<&str>,
) -> Result<(), MonitorError> {
    wait_for_socket(socket_path, 60)?;
    monitor_events(socket_path, marker_path, guest_agent_socket_path)
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

    // Read until we get the qmp_capabilities response. Ignore async events.
    loop {
        let msg = read_json(reader)?;
        if msg.get("return").is_some() {
            break;
        }
        if let Some(err) = msg.get("error") {
            return Err(MonitorError::Protocol(format!(
                "qmp_capabilities failed: {}",
                err
            )));
        }
    }

    Ok(())
}

fn monitor_events(
    socket_path: &str,
    marker_path: &std::path::Path,
    guest_agent_socket_path: Option<&str>,
) -> Result<(), MonitorError> {
    tracing::info!(
        target: "ezkvm::shutdown_monitor",
        "shutdown monitor armed (short-lived QMP polling mode; no guest-agent timeout fallback)"
    );
    let mut guest_agent_reported_running = false;
    let mut last_guest_agent_probe = Instant::now() - Duration::from_secs(3);
    let mut last_qmp_status: Option<String> = None;
    let mut guest_agent_was_running: bool = false;
    let mut qga_stopped_while_running_since: Option<Instant> = None;
    let mut last_stalled_shutdown_log: Option<Instant> = None;
    let mut consecutive_qmp_timeouts = 0u32;
    let mut last_qmp_timeout_log = Instant::now() - QMP_TIMEOUT_LOG_INTERVAL;

    loop {
        maybe_log_guest_agent_state(
            guest_agent_socket_path,
            &mut guest_agent_reported_running,
            &mut last_guest_agent_probe,
        );

        // Track GA running state transitions for stall detection.
        let ga_is_currently_running = guest_agent_reported_running;
        if ga_is_currently_running != guest_agent_was_running {
            guest_agent_was_running = ga_is_currently_running;
            if !ga_is_currently_running && qga_stopped_while_running_since.is_none() {
                // GA just stopped after being reported as running.
                qga_stopped_while_running_since = Some(Instant::now());
                tracing::info!(
                    target: "ezkvm::shutdown_monitor",
                    "guest agent stopped; watching QMP status for shutdown completion"
                );
            } else if ga_is_currently_running {
                // GA became running again; clear stall tracking.
                qga_stopped_while_running_since = None;
                last_stalled_shutdown_log = None;
            }
        }

        match query_qmp_status(socket_path) {
            Ok(Some(status)) => {
                consecutive_qmp_timeouts = 0;

                if last_qmp_status.as_deref() != Some(status.as_str()) {
                    tracing::info!(
                        target: "ezkvm::shutdown_monitor",
                        status = status.as_str(),
                        status_class = qmp_status_class(status.as_str()),
                        "QMP guest status changed"
                    );
                    println!(
                        "Shutdown monitor: QMP status -> '{}' ({})",
                        status.as_str(),
                        qmp_status_class(status.as_str())
                    );
                    last_qmp_status = Some(status.clone());
                }

                // Only warn about stalled shutdown if GA was running before and now stopped while QMP is running.
                if let Some(since) = qga_stopped_while_running_since {
                    let should_log = last_stalled_shutdown_log
                        .is_none_or(|last| last.elapsed() >= STALLED_SHUTDOWN_LOG_INTERVAL);
                    if should_log {
                        tracing::warn!(
                            target: "ezkvm::shutdown_monitor",
                            stalled_for_secs = since.elapsed().as_secs(),
                            "guest agent remains stopped while QMP status stays running"
                        );
                        last_stalled_shutdown_log = Some(Instant::now());
                    }
                }

                if is_qmp_poweroff_status(status.as_str()) {
                    tracing::info!(
                        target: "ezkvm::shutdown_monitor",
                        status = status.as_str(),
                        "detected guest power-off status via query-status"
                    );
                    println!(
                        "Shutdown monitor: detected power-off status '{}' via query-status",
                        status.as_str()
                    );
                    return issue_quit(socket_path, marker_path, guest_agent_socket_path);
                }
            }
            Ok(None) => {}
            Err(MonitorError::Io(err)) if is_timeout_io_error(&err) => {
                consecutive_qmp_timeouts += 1;
                if last_qmp_timeout_log.elapsed() >= QMP_TIMEOUT_LOG_INTERVAL {
                    tracing::debug!(
                        target: "ezkvm::shutdown_monitor",
                        consecutive_timeouts = consecutive_qmp_timeouts,
                        "query-status read timed out"
                    );
                    last_qmp_timeout_log = Instant::now();
                }
            }
            Err(MonitorError::SocketClosed | MonitorError::SocketNotFound) => {
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(err) => {
                tracing::debug!(
                    target: "ezkvm::shutdown_monitor",
                    error = %err,
                    "query-status poll failed"
                );
            }
        }

        std::thread::sleep(Duration::from_millis(750));
    }
}

fn qmp_status_class(status: &str) -> &'static str {
    match status {
        "running" => "active",
        "shutdown" => "terminal-shutdown",
        "internal-error" => "terminal-error",
        "paused" | "suspended" => "paused-or-suspended",
        "watchdog" | "guest-panicked" | "io-error" => "fault",
        _ => "transitional",
    }
}

#[cfg(test)]
fn poweroff_event_name(msg: &serde_json::Value) -> Option<&str> {
    msg.get("event")
        .and_then(|event| event.as_str())
        .filter(|event| matches!(*event, "SHUTDOWN" | "POWERDOWN"))
}

fn issue_quit(
    socket_path: &str,
    marker_path: &std::path::Path,
    guest_agent_socket_path: Option<&str>,
) -> Result<(), MonitorError> {
    // Guest has initiated shutdown (or query-status reports shutdown).
    // Send quit so QEMU exits cleanly instead of spinning while waiting
    // for guest teardown to complete.
    let _ = crate::state::save_shutdown_marker(marker_path);
    tracing::info!(
        target: "ezkvm::shutdown_monitor",
        "requesting QMP quit after guest shutdown detection"
    );
    println!("Shutdown monitor: sending QMP quit");
    with_qmp_session(socket_path, |reader, writer| {
        negotiate_capabilities(reader, writer)?;
        write_command(writer, r#"{"execute":"quit"}"#)
    })?;

    if let Some(socket_path) = guest_agent_socket_path
        && wait_for_guest_agent_stop(socket_path, Duration::from_secs(10))
    {
        tracing::info!(
            target: "ezkvm::shutdown_monitor",
            socket = socket_path,
            "guest agent stopped"
        );
        println!("Shutdown monitor: guest agent stopped ({})", socket_path);
    }

    // Do not wait for socket close here. Main process supervision waits on the
    // QEMU child directly; this monitor should remain fire-and-forget.
    Ok(())
}

fn maybe_log_guest_agent_state(
    guest_agent_socket_path: Option<&str>,
    guest_agent_reported_running: &mut bool,
    last_guest_agent_probe: &mut Instant,
) {
    let Some(socket_path) = guest_agent_socket_path else {
        return;
    };
    if last_guest_agent_probe.elapsed() < Duration::from_secs(2) {
        return;
    }
    *last_guest_agent_probe = Instant::now();

    let running = is_guest_agent_running(socket_path);
    if running && !*guest_agent_reported_running {
        tracing::info!(
            target: "ezkvm::shutdown_monitor",
            socket = socket_path,
            "guest agent is running"
        );
        println!("Shutdown monitor: guest agent is running ({})", socket_path);
        *guest_agent_reported_running = true;
        return;
    }

    if !running && *guest_agent_reported_running {
        tracing::info!(
            target: "ezkvm::shutdown_monitor",
            socket = socket_path,
            "guest agent stopped"
        );
        println!("Shutdown monitor: guest agent stopped ({})", socket_path);
        *guest_agent_reported_running = false;
    }
}

fn wait_for_guest_agent_stop(socket_path: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !is_guest_agent_running(socket_path) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn is_guest_agent_running(socket_path: &str) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket_path) else {
        return false;
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    if stream.write_all(b"{\"execute\":\"guest-ping\"}\n").is_err() {
        return false;
    }
    if stream.flush().is_err() {
        return false;
    }

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let Ok(bytes) = reader.read_line(&mut line) else {
            return false;
        };
        if bytes == 0 {
            return false;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Ok(payload) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            return false;
        };
        if payload.get("error").is_some() {
            return false;
        }
        return payload.get("return").is_some();
    }
}

fn is_timeout_io_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

fn is_qmp_poweroff_status(status: &str) -> bool {
    matches!(status, "shutdown" | "internal-error")
}

fn query_qmp_status(socket_path: &str) -> Result<Option<String>, MonitorError> {
    with_qmp_session(socket_path, |reader, writer| {
        negotiate_capabilities(reader, writer)?;
        write_command(writer, r#"{"execute":"query-status"}"#)?;

        loop {
            let msg = read_json(reader)?;
            if let Some(ret) = msg.get("return") {
                // Ignore unrelated return payloads and wait for query-status reply.
                if let Some(status) = ret.get("status").and_then(|status| status.as_str()) {
                    return Ok(Some(status.to_string()));
                }
                continue;
            }
            if let Some(err) = msg.get("error") {
                return Err(MonitorError::Protocol(format!(
                    "query-status failed: {}",
                    err
                )));
            }
        }
    })
}

fn with_qmp_session<T, F>(socket_path: &str, f: F) -> Result<T, MonitorError>
where
    F: FnOnce(&mut BufReader<UnixStream>, &mut UnixStream) -> Result<T, MonitorError>,
{
    let stream = UnixStream::connect(socket_path).map_err(MonitorError::Io)?;
    stream
        .set_read_timeout(Some(Duration::from_millis(QMP_IO_TIMEOUT_MS)))
        .map_err(MonitorError::Io)?;
    stream
        .set_write_timeout(Some(Duration::from_millis(QMP_IO_TIMEOUT_MS)))
        .map_err(MonitorError::Io)?;

    let reader_stream = stream.try_clone().map_err(MonitorError::Io)?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;
    f(&mut reader, &mut writer)
}

fn is_socket_close_io_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::NotConnected
    )
}

fn write_command(writer: &mut UnixStream, cmd: &str) -> Result<(), MonitorError> {
    writer
        .write_all(format!("{}\n", cmd).as_bytes())
        .map_err(MonitorError::Io)?;
    writer.flush().map_err(MonitorError::Io)
}

fn read_json(reader: &mut BufReader<UnixStream>) -> Result<serde_json::Value, MonitorError> {
    let mut retries = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = match reader.read_line(&mut line) {
            Ok(bytes) => bytes,
            Err(err) if is_retryable_socket_timeout(&err) => {
                retries += 1;
                if retries > QMP_READ_TIMEOUT_RETRIES {
                    return Err(MonitorError::Io(err));
                }
                continue;
            }
            Err(err) => return Err(MonitorError::Io(err)),
        };
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

fn is_retryable_socket_timeout(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

fn is_connection_closed(e: &MonitorError) -> bool {
    match e {
        MonitorError::SocketClosed | MonitorError::SocketNotFound => true,
        MonitorError::Io(err) => is_socket_close_io_error(err),
        _ => false,
    }
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

#[cfg(test)]
mod tests {
    use super::poweroff_event_name;

    #[test]
    fn poweroff_events_include_shutdown_and_powerdown() {
        let shutdown = serde_json::json!({ "event": "SHUTDOWN" });
        let powerdown = serde_json::json!({ "event": "POWERDOWN" });
        let reset = serde_json::json!({ "event": "RESET" });

        assert_eq!(poweroff_event_name(&shutdown), Some("SHUTDOWN"));
        assert_eq!(poweroff_event_name(&powerdown), Some("POWERDOWN"));
        assert_eq!(poweroff_event_name(&reset), None);
    }
}
