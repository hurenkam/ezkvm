use anyhow::{Result, anyhow};

use crate::config::SerialConfig;

pub(super) fn validate_serial_config(serial: &SerialConfig) -> Result<()> {
    let valid_types = ["pty", "stdio", "file", "socket"];
    if !valid_types.contains(&serial.r#type.as_str()) {
        return Err(anyhow!(
            "Unsupported serial type: {}. Supported: {:?}",
            serial.r#type,
            valid_types
        ));
    }

    match serial.r#type.as_str() {
        "file" => {
            let path = serial.path.as_deref().unwrap_or("").trim();
            if path.is_empty() {
                return Err(anyhow!("Serial file backend requires a non-empty path"));
            }
        }
        "socket" => {
            let host = serial.host.as_deref().unwrap_or("").trim();
            if host.is_empty() {
                return Err(anyhow!("Serial socket backend requires a non-empty host"));
            }

            match serial.socket_port {
                Some(0) | None => {
                    return Err(anyhow!(
                        "Serial socket backend requires a TCP port between 1 and 65535"
                    ));
                }
                Some(_) => {}
            }
        }
        "pty" | "stdio" => {}
        _ => unreachable!(),
    }

    Ok(())
}
