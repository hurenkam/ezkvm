//! Ezkvm YAML import adapter.
//!
//! Owns YAML → runtime config transformation via direct YAML validation.

use self::options::EzkvmImportOptions;
use super::{ConfigArgs, ConfigImportError, ConfigImporter, RuntimeConfig, read_config_text};

mod diagnostics;
mod options;
mod parsing;
mod validation;

pub(crate) use validation::validate_ezkvm_config;

#[derive(Debug, Default)]
pub struct EzkvmConfigImporter;

impl ConfigImporter for EzkvmConfigImporter {
    type ConfigError = ConfigImportError;

    fn import_config(&self, config_args: ConfigArgs) -> Result<RuntimeConfig, Self::ConfigError> {
        let options = EzkvmImportOptions::parse(config_args)?;
        let source_text = read_config_text(&options.config_path, "ezkvm")?;

        validate_ezkvm_config(&source_text, &options.config_path).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::config_importer::{ConfigArgs, ConfigImportError, ConfigImporter};

    use super::{EzkvmConfigImporter, options::EzkvmImportOptions};

    fn valid_ezkvm_config_yaml() -> &'static str {
        r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 8192
"#
    }

    fn import_via_trait_object(
        stage: &dyn ConfigImporter<ConfigError = ConfigImportError>,
        config_args: ConfigArgs,
    ) -> Result<crate::runtime_config::RuntimeConfig, ConfigImportError> {
        stage.import_config(config_args)
    }

    fn write_test_yaml(path: &Path, contents: &str) {
        fs::write(path, contents).expect("test fixture should be written");
    }

    fn unique_temp_dir_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn parse_options_requires_config_path() {
        let error =
            EzkvmImportOptions::parse(ConfigArgs::new(vec![])).expect_err("missing path must fail");
        assert!(matches!(
            error,
            ConfigImportError::MissingConfigPath { importer: "ezkvm" }
        ));
    }

    #[test]
    fn parse_options_accepts_named_config_and_extra_import_args() {
        let options = EzkvmImportOptions::parse(ConfigArgs::new(vec![
            "config=vm.yaml".to_string(),
            "host=/etc/ezkvm/host.yaml".to_string(),
            "profiles=/etc/ezkvm/profiles.d".to_string(),
        ]))
        .expect("named args should parse");

        assert_eq!(options.config_path, PathBuf::from("vm.yaml"));
    }

    #[test]
    fn parse_options_rejects_unknown_named_args() {
        let error = EzkvmImportOptions::parse(ConfigArgs::new(vec![
            "config=vm.yaml".to_string(),
            "storage=/etc/pve/storage.cfg".to_string(),
        ]))
        .expect_err("unknown args must fail");

        assert!(matches!(
            error,
            ConfigImportError::UnexpectedArgs {
                importer: "ezkvm",
                ..
            }
        ));
    }

    #[test]
    fn ezkvm_importer_works_through_trait_object() {
        let stage = EzkvmConfigImporter;
        let temp_dir = unique_temp_dir_path("ezkvm-test");
        fs::create_dir_all(&temp_dir).expect("temp test dir should be created");
        let source_path = temp_dir.join("win11-dev.yaml");
        write_test_yaml(&source_path, valid_ezkvm_config_yaml());
        let config_args = ConfigArgs::new(vec![source_path.to_string_lossy().into_owned()]);

        let result = import_via_trait_object(&stage, config_args);

        assert!(result.is_ok());

        let _ = fs::remove_file(source_path);
        let _ = fs::remove_dir(temp_dir);
    }
}
