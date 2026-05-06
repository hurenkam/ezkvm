use super::*;
use ezkvm::import::proxmox::{
    ImportOutputMode, ImportRunOptions, RuntimeTarget, run_import_from_files,
};
use std::path::Path;

fn with_repo_profiles<T>(run: impl FnOnce() -> T) -> T {
    let old = std::env::var_os("EZKVM_CONFIG");
    let central_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("etc/ezkvm.yaml");

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

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

/// Import a fixture conf with PortableLinux target and build QEMU args.
/// Returns (yaml, flat_args) where flat_args is every "-flag value" pair flattened into strings.
fn run_portable_fixture(conf: &str) -> (String, Vec<String>) {
    let conf_path = fixture_path(conf);

    let result = with_repo_profiles(|| {
        run_import_from_files(
            &conf_path.to_string_lossy(),
            &ImportRunOptions {
                output_path: None,
                storage_path: None,
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: ImportOutputMode::Compact,
                runtime_target: RuntimeTarget::PortableLinux,
            },
        )
        .expect("portable fixture import should succeed")
    });

    let config = with_repo_profiles(|| {
        VmConfig::from_str(&result.yaml)
            .unwrap_or_else(|err| panic!("portable fixture yaml invalid: {err}"))
    });

    let args = QemuManager::new(config, CentralConfig::default())
        .build_command()
        .expect("portable fixture command build should succeed")
        .into_inner();

    (result.yaml, args)
}

#[test]
fn portable_q35_planner_allocates_root_ports_and_uses_ezkvm_readconfig() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let (yaml, args) = run_portable_fixture("proxmox_import/12-portable-q35-hostpci.conf");

    // The planner must inject the portable Q35 readconfig, not the Proxmox one.
    assert!(
        args.contains(&"/usr/share/ezkvm/ezkvm-q35.cfg".to_string()),
        "expected ezkvm-q35.cfg in args; args:\n{}",
        args.join("\n")
    );
    assert!(
        !args.iter().any(|a| a.contains("pve-q35")),
        "portable target must not use pve-q35 readconfig; args:\n{}",
        args.join("\n")
    );

    // Machine string must stay plain q35 — not rewritten to +pve0 form.
    let machine_idx = args.iter().position(|a| a == "-machine");
    let machine_val = machine_idx
        .and_then(|i| args.get(i + 1))
        .expect("-machine arg must be present");
    assert!(
        machine_val.starts_with("type=q35"),
        "machine should start with type=q35; got: {machine_val}"
    );
    assert!(
        !machine_val.contains("+pve"),
        "portable machine must not contain +pve; got: {machine_val}"
    );

    // Topology planner must auto-allocate sequential root ports for PCIe passthrough
    // devices that have no explicit bus in the conf.
    let device_args: Vec<&str> = args
        .iter()
        .filter_map(|a| {
            if a.starts_with("vfio-pci") {
                Some(a.as_str())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        device_args.len(),
        2,
        "expected 2 vfio-pci device args; got: {device_args:?}"
    );
    let hostpci0_arg = device_args
        .iter()
        .find(|a| a.contains("id=hostpci0"))
        .expect("hostpci0 device arg must be present");
    assert!(
        hostpci0_arg.contains("bus=ich9-pcie-port-1"),
        "hostpci0 must be placed on ich9-pcie-port-1; got: {hostpci0_arg}"
    );

    let hostpci1_arg = device_args
        .iter()
        .find(|a| a.contains("id=hostpci1"))
        .expect("hostpci1 device arg must be present");
    assert!(
        hostpci1_arg.contains("bus=ich9-pcie-port-2"),
        "hostpci1 must be placed on ich9-pcie-port-2; got: {hostpci1_arg}"
    );

    // Legacy pci.N bus references are expected when portable Q35 readconfig
    // defines bridge-backed pci.0..pci.3 buses.
    let raw_pci_bus_args: Vec<&str> = args
        .iter()
        .filter(|a| {
            // Match "bus=pci.N" but not "bus=pcie.N" or "bus=ich9-pcie*"
            let s = a.as_str();
            s.contains("bus=pci.") && !s.contains("bus=pcie.")
        })
        .map(|a| a.as_str())
        .collect();
    assert!(
        !raw_pci_bus_args.is_empty(),
        "expected portable Q35 args to include legacy pci.N bus references when ezkvm-q35.cfg is loaded; full args:\n{}",
        args.join("\n")
    );

    // Sanity-check: the YAML must record bus values for the host PCI entries.
    assert!(
        yaml.contains("ich9-pcie-port-1"),
        "YAML should contain ich9-pcie-port-1 bus assignment"
    );
    assert!(
        yaml.contains("ich9-pcie-port-2"),
        "YAML should contain ich9-pcie-port-2 bus assignment"
    );
}
