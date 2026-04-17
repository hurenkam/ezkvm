// Temporary over-size rationale (B-29): high-level import flow and its test suite
// remain in one module. Closure target is <=250 lines by extracting validation/
// write-output helpers and moving heavy test setup into dedicated test modules.
use super::{
    ImportError,
    mapper::{
        MappingWarning, map_proxmox_to_canonical_yaml, map_proxmox_to_canonical_yaml_with_storage,
    },
    parser::parse_proxmox_config,
    profile_compact::compact_profile_owned_fields,
    storage_parser::parse_proxmox_storage_config,
    yaml_compact::compact_sequence_mappings,
};
use crate::config::{VmConfig, validation};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ImportRunOptions {
    pub output_path: Option<String>,
    pub storage_path: Option<String>,
    pub strict: bool,
    pub dry_run: bool,
    pub compact_lists: bool,
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

    let storage_config = if let Some(storage_path) = options.storage_path.as_deref() {
        let storage_text = std::fs::read_to_string(storage_path).map_err(|e| {
            ImportError::ParseError(format!(
                "unable to read storage file '{}': {}",
                storage_path, e
            ))
        })?;
        Some(parse_proxmox_storage_config(&storage_text)?)
    } else {
        None
    };

    let parsed = parse_proxmox_config(&input)?;
    let mapped = match storage_config.as_ref() {
        Some(storage_config) => {
            map_proxmox_to_canonical_yaml_with_storage(&parsed, Some(storage_config))?
        }
        None => map_proxmox_to_canonical_yaml(&parsed)?,
    };

    let config = VmConfig::from_str(&mapped.yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "generated canonical YAML failed to deserialize or merge profiles: {e}"
        ))
    })?;
    validation::validate_config(&config).map_err(|e| {
        ImportError::ParseError(format!("generated canonical YAML failed validation: {e}"))
    })?;

    if options.strict && !mapped.warnings.is_empty() {
        return Err(ImportError::ParseError(format!(
            "strict import failed due to {} warning(s): {}",
            mapped.warnings.len(),
            format_warnings(&mapped.warnings)
        )));
    }

    let compacted_profiles_yaml = compact_profile_owned_fields(&mapped.yaml)?;

    let rendered_yaml = if options.compact_lists {
        compact_sequence_mappings(&compacted_profiles_yaml)?
    } else {
        compacted_profiles_yaml
    };

    let output_path = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(input_path));

    if !options.dry_run {
        std::fs::write(&output_path, &rendered_yaml).map_err(|e| {
            ImportError::ParseError(format!(
                "unable to write output file '{}': {}",
                output_path, e
            ))
        })?;
    }

    Ok(ImportRunResult {
        output_path,
        yaml: rendered_yaml,
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

fn format_warnings(warnings: &[MappingWarning]) -> String {
    warnings
        .iter()
        .map(|warning| format!("{}: {}", warning.source_field, warning.message))
        .collect::<Vec<_>>()
        .join("; ")
}


#[cfg(test)]
mod tests {
    use super::{ImportRunOptions, run_import_from_files};
    use crate::test_support::env_lock;
    use std::path::PathBuf;

    fn with_repo_profiles<T>(run: impl FnOnce() -> T) -> T {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let old = std::env::var_os("EZKVM_CONFIG");
        let central_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("etc/ezkvm.yaml");

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_path);
        }

        let result = run();

        unsafe {
            match old {
                Some(value) => std::env::set_var("EZKVM_CONFIG", value),
                None => std::env::remove_var("EZKVM_CONFIG"),
            }
        }

        result
    }

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
        with_repo_profiles(|| {
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
                compact_lists: false,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert!(result.yaml.contains("name: vm-dry"));
            assert!(!output_path.exists());
        });
    }

    #[test]
    fn strict_mode_fails_when_warnings_exist() {
        with_repo_profiles(|| {
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
                compact_lists: false,
            };

            let err = run_import_from_files(&input_path.to_string_lossy(), &options)
                .expect_err("must fail");
            assert!(err.to_string().contains("strict import failed"));
            assert!(err.to_string().contains("arch"));
        });
    }

    #[test]
    fn writes_output_file_when_not_dry_run() {
        with_repo_profiles(|| {
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
                compact_lists: false,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert_eq!(result.output_path, output_path.to_string_lossy());
            assert!(output_path.exists());
        });
    }

    #[test]
    fn resolves_storage_references_when_storage_cfg_is_provided() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-storage-resolution");
            let input_path = dir.join("108.conf");
            let storage_path = dir.join("storage.cfg");

            std::fs::write(
                &input_path,
                "name: vm-storage\nmemory: 2048\ncores: 2\nscsi0: vm1-pool:vm-108-boot,discard=on\n",
            )
            .expect("write input");
            std::fs::write(
                &storage_path,
                "lvmthin: vm1-pool\n    thinpool pool\n    vgname vm1\n    content images,rootdir\n",
            )
            .expect("write storage");

            let options = ImportRunOptions {
                output_path: None,
                storage_path: Some(storage_path.to_string_lossy().to_string()),
                strict: false,
                dry_run: true,
                compact_lists: false,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert!(result.yaml.contains("path: /dev/vm1/vm-108-boot"));
        });
    }

    #[test]
    fn resolves_zfspool_references_when_storage_cfg_is_provided() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-zfspool-resolution");
            let input_path = dir.join("500.conf");
            let storage_path = dir.join("storage.cfg");

            std::fs::write(
                &input_path,
                "name: vm-zfs\nmemory: 2048\ncores: 2\nscsi0: local-zfs:vm-500-disk-0,discard=on\n",
            )
            .expect("write input");
            std::fs::write(
                &storage_path,
                "zfspool: local-zfs\n    pool rpool/data\n    content images,rootdir\n",
            )
            .expect("write storage");

            let options = ImportRunOptions {
                output_path: None,
                storage_path: Some(storage_path.to_string_lossy().to_string()),
                strict: false,
                dry_run: true,
                compact_lists: false,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert!(
                result
                    .yaml
                    .contains("path: /dev/zvol/rpool/data/vm-500-disk-0")
            );
        });
    }
}
