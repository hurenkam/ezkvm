use super::runtime;

fn base_config() -> crate::config::VmConfig {
    crate::config::VmConfig::from_str(
        r#"
name: "test-vm"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    memory:
        size: 1024
        ivshmem:
            enabled: true
            size: 128
            id: "ivshmem0"
            mem_path: "/dev/kvmfr0"
    cpu:
        vcpus: 1
        model: "host"
host:
    pci:
        - device: "0000:03:00.0"
          id: "hostpci0"
          x_vga: true

spice:
    enabled: true
    port: 5903
    addr: "0.0.0.0"
        "#,
    )
    .unwrap()
}

mod looking_glass;
mod remote_viewer;
mod runtime_aux;

#[test]
fn parses_import_qemu_cmd_defaults() {
    use clap::Parser;

    let cli =
        crate::cli::Cli::try_parse_from(["ezkvm", "import-qemu-cmd", "input/felucia/108.qemu.cmd"])
            .expect("cli parse should succeed");

    match cli.command {
        crate::cli::Commands::ImportQemuCmd {
            input,
            output,
            dry_run,
            strict,
            output_mode,
            runtime_target,
        } => {
            assert_eq!(input, "input/felucia/108.qemu.cmd");
            assert_eq!(output, None);
            assert!(!dry_run);
            assert!(!strict);
            assert_eq!(output_mode, crate::cli::ImportOutputModeArg::Compact);
            assert_eq!(
                runtime_target,
                crate::cli::types::RuntimeTargetArg::PortableLinux
            );
        }
        _ => panic!("expected ImportQemuCmd command variant"),
    }
}

#[test]
fn parses_import_qemu_cmd_with_flags() {
    use clap::Parser;

    let cli = crate::cli::Cli::try_parse_from([
        "ezkvm",
        "import-qemu-cmd",
        "input/coruscant/505.qemu.cmd",
        "--output",
        "505.yaml",
        "--dry-run",
        "--strict",
        "--output-mode",
        "debug",
        "--runtime-target",
        "proxmox-parity",
    ])
    .expect("cli parse should succeed");

    match cli.command {
        crate::cli::Commands::ImportQemuCmd {
            input,
            output,
            dry_run,
            strict,
            output_mode,
            runtime_target,
        } => {
            assert_eq!(input, "input/coruscant/505.qemu.cmd");
            assert_eq!(output.as_deref(), Some("505.yaml"));
            assert!(dry_run);
            assert!(strict);
            assert_eq!(output_mode, crate::cli::ImportOutputModeArg::Debug);
            assert_eq!(
                runtime_target,
                crate::cli::types::RuntimeTargetArg::ProxmoxParity
            );
        }
        _ => panic!("expected ImportQemuCmd command variant"),
    }
}
