use anyhow::{Result, anyhow};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) fn ensure_run_dir(
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<PathBuf> {
    let run_dir = crate::state::resolve_runtime_root(None, central_config, runtime_overrides)?;

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
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> String {
    let vm_state_path = config
        .system_tpm()
        .and_then(|tpm| tpm.state_path.as_deref());

    crate::state::resolve_tpm_socket_path(
        &config.name,
        vm_state_path,
        central_config,
        runtime_overrides,
    )
    .unwrap_or_else(|_| format!("/tmp/ezkvm/{}.swtpm", config.name))
}

pub(super) fn resolve_swtpm_log_path(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    run_dir: &Path,
) -> PathBuf {
    let base_dir = config
        .options
        .log_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            central_config
                .host_capabilities
                .runtime
                .log_dir
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| {
            central_config
                .locations
                .log_dir
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| run_dir.to_path_buf());

    base_dir.join(format!("{}-swtpm.log", config.name))
}

pub(super) fn build_tpmstate_arg(
    tpm: &crate::config::TpmConfig,
    run_dir: &Path,
    create_default_dir: bool,
) -> Result<String> {
    if let Some(uri) = tpm.state_backend_uri.as_deref() {
        let normalized = normalize_tpm_backend_uri(uri);

        let mut backend = if normalized.starts_with('/') {
            format!("backend-uri=file://{}", normalized)
        } else {
            format!("backend-uri={}", normalized)
        };
        if !backend.contains(",mode=") && should_append_default_backend_mode(&normalized) {
            backend.push_str(",mode=0600");
        }
        return Ok(backend);
    }

    build_tpmstate_dir_arg(tpm, run_dir, create_default_dir)
}

fn build_tpmstate_dir_arg(
    tpm: &crate::config::TpmConfig,
    run_dir: &Path,
    create_default_dir: bool,
) -> Result<String> {
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

fn normalize_tpm_backend_uri(uri: &str) -> String {
    let trimmed = uri.trim();
    let without_prefix = trimmed.strip_prefix("backend-uri=").unwrap_or(trimmed);
    let no_options = without_prefix.split(',').next().unwrap_or(without_prefix);
    if let Some(stripped) = no_options.strip_prefix("file://dev/") {
        return format!("file:///dev/{}", stripped);
    }
    no_options.to_string()
}

fn should_append_default_backend_mode(normalized_backend_uri: &str) -> bool {
    let Some(local_path) = local_backend_path(normalized_backend_uri) else {
        return true;
    };

    // Treat /dev/* backend paths as device-node style backends even if the
    // current host cannot stat the exact path. This keeps behavior stable
    // across environments and avoids best-effort chmod on device namespaces.
    if local_path.starts_with("/dev/") {
        return false;
    }

    // For device nodes, swtpm mode changes can fail for unprivileged users even when
    // read/write access is granted through group permissions.
    if let Ok(metadata) = std::fs::metadata(&local_path) {
        let file_type = metadata.file_type();
        if file_type.is_block_device() || file_type.is_char_device() {
            return false;
        }
    }

    true
}

fn local_backend_path(normalized_backend_uri: &str) -> Option<PathBuf> {
    if normalized_backend_uri.starts_with('/') {
        return Some(PathBuf::from(normalized_backend_uri));
    }

    let uri = normalized_backend_uri.strip_prefix("file://")?;
    if !uri.starts_with('/') {
        return None;
    }

    Some(PathBuf::from(uri))
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

    #[test]
    fn keeps_backend_uri_when_local_path_exists() {
        let tpm = TpmConfig {
            version: "2.0".to_string(),
            backend: "emulator".to_string(),
            state_path: None,
            state_dir: None,
            state_backend_uri: Some("file:///etc/hosts".to_string()),
            model: "tpm-tis".to_string(),
        };

        let arg = build_tpmstate_arg(&tpm, Path::new("/unused"), true)
            .expect("building tpmstate arg should succeed");
        assert_eq!(arg, "backend-uri=file:///etc/hosts,mode=0600");
    }

    #[test]
    fn keeps_backend_uri_when_local_path_is_missing() {
        let tpm = TpmConfig {
            version: "2.0".to_string(),
            backend: "emulator".to_string(),
            state_path: None,
            state_dir: None,
            state_backend_uri: Some("file:///definitely/missing/tpmstate".to_string()),
            model: "tpm-tis".to_string(),
        };

        let arg = build_tpmstate_arg(&tpm, Path::new("/unused"), true)
            .expect("building tpmstate arg should succeed");
        assert_eq!(
            arg,
            "backend-uri=file:///definitely/missing/tpmstate,mode=0600"
        );
    }

    #[test]
    fn strips_backend_uri_prefix_and_options() {
        let tpm = TpmConfig {
            version: "2.0".to_string(),
            backend: "emulator".to_string(),
            state_path: None,
            state_dir: None,
            state_backend_uri: Some(
                "backend-uri=file:///dev/vm1/vm-108-tpmstate,mode=0600".to_string(),
            ),
            model: "tpm-tis".to_string(),
        };

        let arg = build_tpmstate_arg(&tpm, Path::new("/unused"), true)
            .expect("building tpmstate arg should succeed");
        assert_eq!(arg, "backend-uri=file:///dev/vm1/vm-108-tpmstate");
    }

    #[test]
    fn omits_default_mode_for_device_backend_uri() {
        let tpm = TpmConfig {
            version: "2.0".to_string(),
            backend: "emulator".to_string(),
            state_path: None,
            state_dir: None,
            state_backend_uri: Some("file:///dev/null".to_string()),
            model: "tpm-tis".to_string(),
        };

        let arg = build_tpmstate_arg(&tpm, Path::new("/unused"), true)
            .expect("building tpmstate arg should succeed");
        assert_eq!(arg, "backend-uri=file:///dev/null");
    }
}
