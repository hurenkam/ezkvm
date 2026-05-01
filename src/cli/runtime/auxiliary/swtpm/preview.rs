use anyhow::{Result, anyhow};
use std::path::Path;

use super::helpers::{build_tpmstate_arg, resolve_swtpm_log_path, resolve_tpm_socket_path};

pub(crate) fn build_swtpm_launch_preview(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<Option<String>> {
    let tpm = match config.system_tpm() {
        Some(tpm) if tpm.backend == "emulator" => tpm,
        _ => return Ok(None),
    };

    if tpm.state_path.is_some() {
        return Ok(None);
    }

    let placement_mode =
        crate::state::resolve_tpm_placement_mode(central_config, runtime_overrides);
    if placement_mode == crate::state::TpmPlacementMode::StateFile {
        return Ok(None);
    }

    let swtpm_path = match crate::state::resolve_swtpm_binary(central_config, runtime_overrides) {
        Some(path) => path,
        None => {
            return Err(anyhow!(
                "TPM emulator backend in socket mode requires --swtpm-binary, host_capabilities.tpm.swtpm_binary, PATH swtpm, or legacy tools.swtpm"
            ));
        }
    };

    let run_dir = crate::state::resolve_runtime_root(None, central_config, runtime_overrides)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "/tmp/ezkvm".to_string());

    let socket_path = resolve_tpm_socket_path(config, central_config, runtime_overrides);
    let pid_path = Path::new(&run_dir).join(format!("{}.swtpm.pid", config.name));
    let log_path = resolve_swtpm_log_path(config, central_config, Path::new(&run_dir));
    let tpmstate_arg = build_tpmstate_arg(tpm, Path::new(&run_dir), false)?;

    let tpm_flag = if tpm.version == "2.0" {
        "--tpm2"
    } else {
        "--tpm"
    };
    let ctrl_arg = format!("type=unixio,path={},mode=0600", socket_path);
    let pid_arg = format!("file={}", pid_path.display());
    let log_arg = format!("file={},level=1", log_path.display());

    let rendered = vec![
        swtpm_path,
        "socket".to_string(),
        tpm_flag.to_string(),
        "--tpmstate".to_string(),
        tpmstate_arg,
        "--ctrl".to_string(),
        ctrl_arg,
        "--pid".to_string(),
        pid_arg,
        "--terminate".to_string(),
        "--log".to_string(),
        log_arg,
        "--daemon".to_string(),
    ];

    Ok(Some(rendered.join(" ")))
}
