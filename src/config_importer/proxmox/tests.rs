use std::fs;
use std::path::PathBuf;

use crate::config_importer::{ConfigArgs, ConfigImportError, ConfigImporter};

use super::{ProxmoxConfigImporter, options::ProxmoxImportOptions};

#[test]
fn parse_options_requires_config_path() {
    let error =
        ProxmoxImportOptions::parse(ConfigArgs::new(vec![])).expect_err("missing path must fail");

    assert!(matches!(
        error,
        ConfigImportError::MissingConfigPath {
            importer: "proxmox"
        }
    ));
}

#[test]
fn parse_options_accepts_named_config_and_extra_import_args() {
    let options = ProxmoxImportOptions::parse(ConfigArgs::new(vec![
        "config=/tmp/108.conf".to_string(),
        "storage=/etc/pve/storage.cfg".to_string(),
    ]))
    .expect("named args should parse");

    assert_eq!(options.config_path, PathBuf::from("/tmp/108.conf"));
}

#[test]
fn parse_options_rejects_unknown_named_args() {
    let error = ProxmoxImportOptions::parse(ConfigArgs::new(vec![
        "config=/tmp/108.conf".to_string(),
        "host=/etc/ezkvm/host.yaml".to_string(),
    ]))
    .expect_err("unknown args must fail");

    assert!(matches!(
        error,
        ConfigImportError::UnexpectedArgs {
            importer: "proxmox",
            ..
        }
    ));
}

#[test]
fn proxmox_importer_rejects_malformed_source_name() {
    let source_path = PathBuf::from("/tmp/wakiza.yaml");
    fs::write(
        &source_path,
        "name: wakiza\nmachine: pc-q35-8.1\ncpu: host\nmemory: 8192\n",
    )
    .expect("fixture write should succeed");
    let config_args = ConfigArgs::new(vec![source_path.to_string_lossy().into_owned()]);

    let err = ProxmoxConfigImporter
        .import_config(config_args)
        .expect_err("non-.conf source names should be rejected");

    assert!(matches!(
        err,
        ConfigImportError::Importer {
            importer: "proxmox",
            ..
        }
    ));
    assert!(
        err.to_string()
            .contains("expected a Proxmox .conf source name")
    );

    let _ = fs::remove_file(source_path);
}
