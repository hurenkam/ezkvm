use super::error::ImportError;
use super::mapper::{CanonicalMappingResult, MappingWarning, map_qemu_cmd_to_canonical_yaml};
use super::model::QemuCmdModel;
use super::parser::parse_qemu_cmd;
use crate::import::common::{
    io_contract::{default_output_path, enforce_strict_mode, write_output_if_needed},
    render::prepend_preamble,
    validate::validate_generated_vm_yaml,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportOutputMode {
    Canonical,
    #[default]
    Compact,
    DebugCanonical,
}

#[derive(Debug, Clone)]
pub struct ImportRunOptions {
    pub output_path: Option<String>,
    pub strict: bool,
    pub dry_run: bool,
    pub output_mode: ImportOutputMode,
}

#[derive(Debug, Clone)]
pub struct ImportRunResult {
    pub output_path: String,
    pub yaml: String,
    pub warnings: Vec<MappingWarning>,
}

pub fn run_import_from_files(
    input_path: &str,
    options: &ImportRunOptions,
) -> Result<ImportRunResult, ImportError> {
    let input = std::fs::read_to_string(input_path).map_err(|e| {
        ImportError::ParseError(format!("unable to read input file '{}': {}", input_path, e))
    })?;

    let parsed = parse_qemu_cmd(&input)?;
    let mapped = map_qemu_cmd_to_canonical_yaml(&parsed)?;

    validate_generated_vm_yaml(&mapped.yaml).map_err(ImportError::ParseError)?;

    enforce_strict_mode(options.strict, &mapped.warnings, |warning| {
        format!("{}: {}", warning.source_field, warning.message)
    })
    .map_err(ImportError::ParseError)?;

    let rendered_yaml = render_export_yaml(&parsed, &mapped, options)?;

    let output_path = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(input_path));

    write_output_if_needed(&output_path, &rendered_yaml, options.dry_run)
        .map_err(ImportError::ParseError)?;

    Ok(ImportRunResult {
        output_path,
        yaml: rendered_yaml,
        warnings: mapped.warnings,
    })
}

fn render_export_yaml(
    parsed: &QemuCmdModel,
    mapped: &CanonicalMappingResult,
    options: &ImportRunOptions,
) -> Result<String, ImportError> {
    let yaml = match options.output_mode {
        ImportOutputMode::Canonical | ImportOutputMode::Compact => mapped.yaml.clone(),
        ImportOutputMode::DebugCanonical => prepend_preamble(
            mapped.yaml.clone(),
            Some(&build_debug_source_comments(parsed, &mapped.warnings)),
        ),
    };

    if yaml.trim().is_empty() {
        return Err(ImportError::ParseError(
            "generated YAML is unexpectedly empty".to_string(),
        ));
    }

    Ok(yaml)
}

fn build_debug_source_comments(parsed: &QemuCmdModel, warnings: &[MappingWarning]) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# from qemu executable: {}", parsed.executable));

    if !warnings.is_empty() {
        lines.push(format!("# mapping warnings: {}", warnings.len()));
        for warning in warnings {
            lines.push(format!(
                "# warning {}: {}",
                warning.source_field, warning.message
            ));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{ImportOutputMode, ImportRunOptions, run_import_from_files};

    fn create_temp_input_file(prefix: &str, content: &str) -> std::path::PathBuf {
        let unique = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be monotonic")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(format!("{unique}.qemu.cmd"));
        std::fs::write(&path, content).expect("temporary input should be writable");
        path
    }

    #[test]
    fn strict_mode_fails_when_warnings_exist() {
        let input_path = create_temp_input_file(
            "qemu-cmd-strict",
            "/usr/bin/kvm -incoming defer -name vm-strict",
        );

        let result = run_import_from_files(
            &input_path.to_string_lossy(),
            &ImportRunOptions {
                output_path: None,
                strict: true,
                dry_run: true,
                output_mode: ImportOutputMode::Canonical,
            },
        );

        let error = result.expect_err("strict mode should fail on warnings");
        assert!(error.to_string().contains("strict import failed"));

        let _ = std::fs::remove_file(input_path);
    }

    #[test]
    fn dry_run_returns_yaml_without_writing_output_file() {
        let input_path = create_temp_input_file(
            "qemu-cmd-dry-run",
            "/usr/bin/kvm -name vm-dry-run -m 2048 -smp 2",
        );
        let expected_output = format!(
            "{}.yaml",
            input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("temp file should have a valid stem")
        );

        let result = run_import_from_files(
            &input_path.to_string_lossy(),
            &ImportRunOptions {
                output_path: None,
                strict: false,
                dry_run: true,
                output_mode: ImportOutputMode::Canonical,
            },
        )
        .expect("dry-run import should succeed");

        assert!(result.yaml.contains("name: vm-dry-run"));
        assert_eq!(result.output_path, expected_output);
        assert!(std::path::Path::new(&result.output_path).is_relative());

        let _ = std::fs::remove_file(input_path);
    }

    #[test]
    fn writes_output_file_when_not_dry_run() {
        let input_path = create_temp_input_file(
            "qemu-cmd-write",
            "/usr/bin/kvm -name vm-write -m 1024 -smp 1",
        );

        let output_path = std::env::temp_dir().join(format!(
            "qemu-cmd-write-output-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be monotonic")
                .as_nanos()
        ));

        let result = run_import_from_files(
            &input_path.to_string_lossy(),
            &ImportRunOptions {
                output_path: Some(output_path.to_string_lossy().to_string()),
                strict: false,
                dry_run: false,
                output_mode: ImportOutputMode::DebugCanonical,
            },
        )
        .expect("non-dry-run import should succeed");

        let written = std::fs::read_to_string(&output_path).expect("output file should exist");
        assert_eq!(written, result.yaml);
        assert!(written.contains("# from qemu executable:"));

        let _ = std::fs::remove_file(input_path);
        let _ = std::fs::remove_file(output_path);
    }
}
