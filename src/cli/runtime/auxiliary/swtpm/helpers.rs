use anyhow::{Result, anyhow};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) fn ensure_run_dir(central_config: &crate::config::CentralConfig) -> Result<PathBuf> {
    let run_dir = central_config
        .locations
        .run_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/run/ezkvm"));

    std::fs::create_dir_all(&run_dir)?;
    Ok(run_dir)
}

pub(super) fn ensure_socket_parent_dir(socket_path: &str, label: &str) -> Result<()> {
    let parent = Path::new(socket_path).parent().ok_or_else(|| {
        anyhow!(
            "{} '{}' does not have a parent directory",
            label,
            socket_path
        )
    })?;

    std::fs::create_dir_all(parent).map_err(|err| {
        anyhow!(
            "failed to create parent directory for {} '{}': {}",
            label,
            socket_path,
            err
        )
    })
}

pub(super) fn wait_for_unix_socket(
    socket_path: &str,
    timeout: Duration,
    label: &str,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_error = None;

    while Instant::now() < deadline {
        match UnixStream::connect(socket_path) {
            Ok(stream) => {
                drop(stream);
                return Ok(());
            }
            Err(err) => {
                last_error = Some(err);
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    match last_error {
        Some(err) => Err(anyhow!(
            "timed out waiting for {} '{}' to become ready: {}",
            label,
            socket_path,
            err
        )),
        None => Err(anyhow!(
            "timed out waiting for {} '{}' to become ready",
            label,
            socket_path
        )),
    }
}

pub(super) fn resolve_tpm_socket_path(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> String {
    if let Some(tpm) = config.system_tpm()
        && let Some(state_path) = &tpm.state_path
    {
        return state_path.clone();
    }

    if let Some(run_dir) = &central_config.locations.run_dir {
        return format!("{}/{}.swtpm", run_dir, config.name);
    }

    format!("/var/run/qemu-server/{}.swtpm", config.name)
}

pub(super) fn build_tpmstate_arg(
    tpm: &crate::config::TpmConfig,
    run_dir: &Path,
    create_default_dir: bool,
) -> Result<String> {
    if let Some(uri) = tpm.state_backend_uri.as_deref() {
        let trimmed = uri.trim();
        let normalized = if let Some(stripped) = trimmed.strip_prefix("file://dev/") {
            format!("file:///dev/{}", stripped)
        } else {
            trimmed.to_string()
        };
        let mut backend = if normalized.starts_with('/') {
            format!("backend-uri=file://{}", normalized)
        } else {
            format!("backend-uri={}", normalized)
        };
        if !backend.contains(",mode=") {
            backend.push_str(",mode=0600");
        }
        return Ok(backend);
    }

    let state_dir_path: PathBuf = if let Some(explicit) = tpm.state_dir.as_deref() {
        explicit.into()
    } else {
        let default = run_dir.join("tpm-state");
        if create_default_dir {
            std::fs::create_dir_all(&default)?;
        }
        default
    };

    if create_default_dir {
        std::fs::create_dir_all(&state_dir_path)?;
    }

    Ok(format!("dir={}", state_dir_path.display()))
}

#[cfg(test)]
mod tests {
    use super::build_tpmstate_arg;
    use crate::config::TpmConfig;
    use std::path::Path;

    fn unique_temp_path(suffix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("ezkvm-{}-{}", suffix, nanos))
    }

    #[test]
    fn creates_explicit_state_dir_when_requested() {
        let explicit_dir = unique_temp_path("swtpm-explicit");
        let tpm = TpmConfig {
            version: "2.0".to_string(),
            backend: "emulator".to_string(),
            state_path: None,
            state_dir: Some(explicit_dir.to_string_lossy().to_string()),
            state_backend_uri: None,
            model: "tpm-tis".to_string(),
        };

        let arg = build_tpmstate_arg(&tpm, Path::new("/unused"), true)
            .expect("building tpmstate arg should succeed");
        assert!(arg.starts_with("dir="));
        assert!(explicit_dir.is_dir());

        let _ = std::fs::remove_dir_all(&explicit_dir);
    }

    #[test]
    fn does_not_create_explicit_state_dir_for_preview_mode() {
        let explicit_dir = unique_temp_path("swtpm-explicit-preview");
        let tpm = TpmConfig {
            version: "2.0".to_string(),
            backend: "emulator".to_string(),
            state_path: None,
            state_dir: Some(explicit_dir.to_string_lossy().to_string()),
            state_backend_uri: None,
            model: "tpm-tis".to_string(),
        };

        let arg = build_tpmstate_arg(&tpm, Path::new("/unused"), false)
            .expect("building tpmstate arg should succeed");
        assert!(arg.starts_with("dir="));
        assert!(!explicit_dir.exists());
    }
}
