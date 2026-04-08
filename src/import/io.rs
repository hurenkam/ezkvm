use super::mapper::map_to_ezkvm_yaml;
use super::proxmox_parser::parse_proxmox_config;
use super::{EzkvmImportResult, ImportError};
use crate::vm::config::Config;
use std::fs;
use std::path::Path;

pub fn import_from_text(input: &str, name_override: Option<&str>) -> Result<EzkvmImportResult, ImportError> {
    let proxmox = parse_proxmox_config(input)?;
    let result = map_to_ezkvm_yaml(&proxmox, name_override)?;

    serde_yaml::from_str::<Config>(&result.yaml)
        .map_err(|e| ImportError::ValidationError(format!("generated yaml is not valid ezkvm config: {}", e)))?;

    Ok(result)
}

pub fn import_from_file(
    input_path: &str,
    output_path: Option<&str>,
    name_override: Option<&str>,
    strict: bool,
    dry_run: bool,
) -> Result<String, ImportError> {
    let content = fs::read_to_string(input_path)
        .map_err(|e| ImportError::IoError(format!("unable to read '{}': {}", input_path, e)))?;

    let result = import_from_text(&content, name_override)?;

    if strict && !result.warnings.is_empty() {
        return Err(ImportError::ValidationError(format!(
            "strict import failed: {} warning(s): {}",
            result.warnings.len(),
            result.warnings.join("; ")
        )));
    }

    let output_path = output_path
        .map(|x| x.to_string())
        .unwrap_or_else(|| default_output_path(input_path, name_override));

    if dry_run {
        return Ok(build_summary(
            &output_path,
            &result,
            true,
            Some(&result.yaml),
        ));
    }

    fs::write(&output_path, &result.yaml)
        .map_err(|e| ImportError::IoError(format!("unable to write '{}': {}", output_path, e)))?;

    Ok(build_summary(&output_path, &result, false, None))
}

fn default_output_path(input_path: &str, name_override: Option<&str>) -> String {
    let base_name = match name_override {
        Some(name) => name.to_string(),
        None => {
            let path = Path::new(input_path);
            path.file_stem()
                .and_then(|x| x.to_str())
                .unwrap_or("imported-vm")
                .to_string()
        }
    };

    format!("{}.yaml", base_name)
}

fn build_summary(
    output_path: &str,
    result: &EzkvmImportResult,
    dry_run: bool,
    yaml: Option<&str>,
) -> String {
    let mut lines = vec![
        format!("import {}", if dry_run { "dry-run" } else { "complete" }),
        format!("output: {}", output_path),
        format!("mapped: {}", result.mapped_keys.len()),
        format!("skipped: {}", result.skipped_keys.len()),
        format!("warnings: {}", result.warnings.len()),
    ];

    if !result.warnings.is_empty() {
        lines.push("warning details:".to_string());
        for warning in &result.warnings {
            lines.push(format!("- {}", warning));
        }
    }

    if let Some(yaml) = yaml {
        lines.push("generated yaml:".to_string());
        lines.push(yaml.to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{import_from_file, import_from_text};
    use std::fs;
    use tempfile::tempdir;

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

    #[test]
    fn test_import_from_file_writes_output() {
        let temp = tempdir().unwrap();
        let input_path = temp.path().join("vm100.conf");
        let output_path = temp.path().join("vm100.yaml");

        fs::write(
            &input_path,
            r#"
            name: vm-100
            memory: 4096
            cores: 2
            sockets: 1
            bios: ovmf
            scsi0: /dev/vm1/vm-100-disk-0,discard=on
            net0: virtio=BC:24:11:FF:76:89,bridge=vmbr0
            "#,
        )
        .unwrap();

        let summary = import_from_file(
            input_path.to_str().unwrap(),
            Some(output_path.to_str().unwrap()),
            None,
            false,
            false,
        )
        .unwrap();

        assert!(summary.contains("import complete"));
        let yaml = fs::read_to_string(output_path).unwrap();
        assert!(yaml.contains("general:"));
    }

    #[test]
    fn test_import_from_file_strict_fails_on_warning() {
        let temp = tempdir().unwrap();
        let input_path = temp.path().join("vm101.conf");

        fs::write(
            &input_path,
            r#"
            name: vm-101
            virtio0: local-lvm:vm-101-disk-0
            "#,
        )
        .unwrap();

        let err = import_from_file(input_path.to_str().unwrap(), None, None, true, true).unwrap_err();
        assert!(format!("{:?}", err).contains("strict import failed"));
    }
}
