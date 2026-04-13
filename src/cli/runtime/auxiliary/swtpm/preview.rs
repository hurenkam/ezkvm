use anyhow::{Result, anyhow};
use std::path::Path;

use super::helpers::{build_tpmstate_arg, resolve_tpm_socket_path};

pub(crate) fn build_swtpm_launch_preview(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<Option<String>> {
    let tpm = match config.system_tpm() {
        Some(tpm) if tpm.backend == "emulator" => tpm,
        _ => return Ok(None),
    };

    let swtpm_path = match &central_config.tools.swtpm {
        Some(path) => path,
        None => {
            return Err(anyhow!(
                "TPM emulator backend requires tools.swtpm to be configured in the central config"
            ));
        }
    };

    let run_dir = central_config
        .locations
        .run_dir
        .as_deref()
        .unwrap_or("/var/run/ezkvm");

    let socket_path = resolve_tpm_socket_path(config, central_config);
    let pid_path = Path::new(run_dir).join(format!("{}.swtpm.pid", config.name));
    let log_path = Path::new(run_dir).join(format!("{}-swtpm.log", config.name));

    let tpmstate_arg = build_tpmstate_arg(tpm, Path::new(run_dir), false)?;

    let tpm_flag = if tpm.version == "2.0" {
        "--tpm2"
    } else {
        "--tpm"
    };
    let ctrl_arg = format!("type=unixio,path={},mode=0600", socket_path);
    let pid_arg = format!("file={}", pid_path.display());
    let log_arg = format!("file={},level=1", log_path.display());

    let rendered = vec![
        swtpm_path.clone(),
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
