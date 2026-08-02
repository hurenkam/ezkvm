use std::{
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub enum ReadinessError {
    #[error("socket at {path} did not become ready within {timeout_secs}s")]
    Timeout { path: String, timeout_secs: u64 },
}

pub fn wait_for_socket(path: &str, timeout: Duration) -> Result<(), ReadinessError> {
    let deadline = Instant::now() + timeout;
    let poll_interval = Duration::from_millis(50);

    loop {
        if UnixStream::connect(path).is_ok() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(ReadinessError::Timeout {
                path: path.to_string(),
                timeout_secs: timeout.as_secs(),
            });
        }

        std::thread::sleep(poll_interval);
    }
}
