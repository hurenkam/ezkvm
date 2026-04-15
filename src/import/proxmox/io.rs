use super::{
    ImportError, map_proxmox_to_canonical_yaml, parse_proxmox_config, parse_proxmox_storage_config,
};
use crate::config::{VmConfig, validation};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ImportRunOptions {
    pub output_path: Option<String>,
    pub storage_path: Option<String>,
    pub strict: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ImportRunResult {
    pub output_path: String,
    pub yaml: String,
    pub warnings: Vec<String>,
}

pub fn run_import_from_files(
    input_path: &str,
    options: &ImportRunOptions,
) -> Result<ImportRunResult, ImportError> {
    let input = std::fs::read_to_string(input_path).map_err(|e| {
        ImportError::ParseError(format!("unable to read input file '{}': {}", input_path, e))
    })?;

    if let Some(storage_path) = options.storage_path.as_deref() {
        let storage_text = std::fs::read_to_string(storage_path).map_err(|e| {
            ImportError::ParseError(format!(
                "unable to read storage file '{}': {}",
                storage_path, e
            ))
        })?;
        let _ = parse_proxmox_storage_config(&storage_text)?;
    }

    let parsed = parse_proxmox_config(&input)?;
    let mapped = map_proxmox_to_canonical_yaml(&parsed)?;

    let config: VmConfig = serde_yaml::from_str(&mapped.yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "generated canonical YAML failed to deserialize: {e}"
        ))
    })?;
    validation::validate_config(&config).map_err(|e| {
        ImportError::ParseError(format!("generated canonical YAML failed validation: {e}"))
    })?;

    if options.strict && !mapped.warnings.is_empty() {
        return Err(ImportError::ParseError(format!(
            "strict import failed due to {} warning(s): {}",
            mapped.warnings.len(),
            mapped.warnings.join("; ")
        )));
    }

    let output_path = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(input_path));

    if !options.dry_run {
        std::fs::write(&output_path, &mapped.yaml).map_err(|e| {
            ImportError::ParseError(format!(
                "unable to write output file '{}': {}",
                output_path, e
            ))
        })?;
    }

    Ok(ImportRunResult {
        output_path,
        yaml: mapped.yaml,
        warnings: mapped.warnings,
    })
}

fn default_output_path(input_path: &str) -> String {
    let input = Path::new(input_path);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("imported-vm");

    format!("{}.yaml", stem)
}

#[cfg(test)]
mod tests {
    use super::{ImportRunOptions, run_import_from_files};
    use std::path::PathBuf;

    fn create_temp_test_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        dir.push(format!("ezkvm-{}-{}", name, nanos));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn dry_run_does_not_write_output_file() {
        let dir = create_temp_test_dir("import-dry-run");
        let input_path = dir.join("100.conf");
        std::fs::write(
            &input_path,
            "name: vm-dry\nmemory: 2048\ncores: 2\nscsi0: local-lvm:disk0,size=10G\n",
        )
        .expect("write input");

        let output_path = dir.join("result.yaml");
        let options = ImportRunOptions {
            output_path: Some(output_path.to_string_lossy().to_string()),
            storage_path: None,
            strict: false,
            dry_run: true,
        };

        let result =
            run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
        assert!(result.yaml.contains("name: vm-dry"));
        assert!(!output_path.exists());
    }

    #[test]
    fn strict_mode_fails_when_warnings_exist() {
        let dir = create_temp_test_dir("import-strict");
        let input_path = dir.join("101.conf");
        std::fs::write(
            &input_path,
            "name: vm-strict\narch: sparc\nmemory: 2048\ncores: 2\n",
        )
        .expect("write input");

        let options = ImportRunOptions {
            output_path: None,
            storage_path: None,
            strict: true,
            dry_run: true,
        };

        let err =
            run_import_from_files(&input_path.to_string_lossy(), &options).expect_err("must fail");
        assert!(err.to_string().contains("strict import failed"));
    }

    #[test]
    fn writes_output_file_when_not_dry_run() {
        let dir = create_temp_test_dir("import-write");
        let input_path = dir.join("102.conf");
        std::fs::write(
            &input_path,
            "name: vm-write\nmemory: 2048\ncores: 2\nscsi0: local-lvm:disk0,size=10G\n",
        )
        .expect("write input");

        let output_path = dir.join("custom.yaml");
        let options = ImportRunOptions {
            output_path: Some(output_path.to_string_lossy().to_string()),
            storage_path: None,
            strict: false,
            dry_run: false,
        };

        let result =
            run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
        assert_eq!(result.output_path, output_path.to_string_lossy());
        assert!(output_path.exists());
    }
}
