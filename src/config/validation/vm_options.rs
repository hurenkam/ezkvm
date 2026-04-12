use anyhow::{Result, anyhow};

use crate::config::VmOptions;

pub(crate) fn validate_vm_options(options: &VmOptions) -> Result<()> {
    for global in &options.global_options {
        if global.trim().is_empty() {
            return Err(anyhow!("Global QEMU options cannot be empty"));
        }
    }

    if let Some(rtc) = &options.rtc {
        if let Some(base) = &rtc.base {
            let valid = ["utc", "localtime"];
            if !valid.contains(&base.as_str()) {
                return Err(anyhow!(
                    "Unsupported RTC base: {}. Supported: {:?}",
                    base,
                    valid
                ));
            }
        }

        if let Some(driftfix) = &rtc.driftfix {
            let valid = ["none", "slew"];
            if !valid.contains(&driftfix.as_str()) {
                return Err(anyhow!(
                    "Unsupported RTC driftfix: {}. Supported: {:?}",
                    driftfix,
                    valid
                ));
            }
        }
    }

    if let Some(pid_file) = &options.pid_file
        && pid_file.trim().is_empty()
    {
        return Err(anyhow!("PID file path cannot be empty"));
    }

    if let Some(log_dir) = &options.log_dir
        && log_dir.trim().is_empty()
    {
        return Err(anyhow!("Log directory cannot be empty"));
    }

    if let Some(log_keep) = options.log_keep
        && log_keep == 0
    {
        return Err(anyhow!("log_keep must be greater than 0"));
    }

    Ok(())
}
