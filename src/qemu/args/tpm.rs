use crate::qemu::types::QemuArgs;

impl QemuArgs {
    /// Add TPM device
    ///
    /// Returns an error if the backend is unsupported, rather than panicking.
    /// Supported backends: "emulator", "passthrough"
    pub fn add_tpm(
        &mut self,
        _version: &str,
        backend: &str,
        socket_path: &str,
        model: &str,
        external_swtpm: bool,
        state_file_mode: bool,
    ) -> Result<(), String> {
        add_tpm_backend_args(self, backend, socket_path, external_swtpm, state_file_mode)?;

        self.push_str("-device");
        self.push(format!("{},tpmdev=tpmdev", model));

        Ok(())
    }
}

fn add_tpm_backend_args(
    args: &mut QemuArgs,
    backend: &str,
    socket_path: &str,
    external_swtpm: bool,
    state_file_mode: bool,
) -> Result<(), String> {
    match backend {
        "emulator" => {
            let chardev_id = "tpmchar";
            args.push_str("-chardev");
            if state_file_mode {
                args.push(format!("tpmemu,id={}", chardev_id));
            } else {
                args.push(build_tpm_chardev_spec(
                    chardev_id,
                    socket_path,
                    external_swtpm,
                ));
            }
            args.push_str("-tpmdev");
            args.push(format!("emulator,id=tpmdev,chardev={}", chardev_id));
            Ok(())
        }
        "passthrough" => {
            args.push_str("-tpmdev");
            args.push("passthrough,id=tpmdev".to_string());
            Ok(())
        }
        _ => Err(format!(
            "Unsupported TPM backend: '{}'. Supported backends: 'emulator', 'passthrough'",
            backend
        )),
    }
}

fn build_tpm_chardev_spec(chardev_id: &str, socket_path: &str, external_swtpm: bool) -> String {
    if external_swtpm {
        format!("socket,id={},path={}", chardev_id, socket_path)
    } else {
        format!(
            "socket,id={},server=on,wait=off,path={}",
            chardev_id, socket_path
        )
    }
}
