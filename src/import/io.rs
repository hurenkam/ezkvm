use super::mapper::map_to_ezkvm_yaml;
use super::proxmox_parser::parse_proxmox_config;
use super::{EzkvmImportResult, ImportError};
use crate::vm::config::Config;

pub fn import_from_text(input: &str, name_override: Option<&str>) -> Result<EzkvmImportResult, ImportError> {
    let proxmox = parse_proxmox_config(input)?;
    let result = map_to_ezkvm_yaml(&proxmox, name_override)?;

    serde_yaml::from_str::<Config>(&result.yaml)
        .map_err(|e| ImportError::ValidationError(format!("generated yaml is not valid ezkvm config: {}", e)))?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::import_from_text;

    #[test]
    fn test_import_from_text_success() {
        let result = import_from_text(
            r#"
            name: vm-100
            memory: 4096
            cores: 2
            sockets: 1
            bios: ovmf
            scsi0: /dev/vm1/vm-100-disk-0,discard=on
            net0: virtio=BC:24:11:FF:76:89,bridge=vmbr0
            "#,
            None,
        )
        .unwrap();

        assert!(result.yaml.contains("general:"));
        assert!(result.yaml.contains("storage:"));
        assert!(result.yaml.contains("network:"));
    }
}
