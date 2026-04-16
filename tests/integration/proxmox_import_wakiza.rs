use super::*;
use ezkvm::import::proxmox::{ImportRunOptions, run_import_from_files};

fn with_repo_profiles<T>(run: impl FnOnce() -> T) -> T {
    let old = std::env::var_os("EZKVM_CONFIG");
    let central_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("etc/ezkvm.yaml");

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

#[test]
fn test_wakiza_import_preserves_key_proxmox_fragments() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let storage_path = format!("{}/input/felucia/storage.cfg", env!("CARGO_MANIFEST_DIR"));

    let result = with_repo_profiles(|| {
        run_import_from_files(
            "input/felucia/108.conf",
            &ImportRunOptions {
                output_path: None,
                storage_path: Some(storage_path),
                strict: false,
                dry_run: true,
                compact_lists: false,
            },
        )
        .expect("wakiza import should succeed")
    });

    assert!(
        result.warnings.is_empty(),
        "unexpected import warnings: {:?}",
        result
            .warnings
            .iter()
            .map(|w| (&w.source_field, &w.message))
            .collect::<Vec<_>>()
    );

    let config = with_repo_profiles(|| {
        VmConfig::from_str(&result.yaml).expect("imported yaml should deserialize")
    });
    let manager = QemuManager::new(config, CentralConfig::default());
    let args = manager
        .build_command()
        .expect("imported command should build");
    let generated = format!(
        "{} {}",
        manager.binary_name(),
        args.iter()
            .map(|arg| arg.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );

    let proxmox_cmd = include_str!("../../input/felucia/108.qemu.cmd");

    let shared_fragments = [
        "if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-108-efidisk,size=540672",
        "type=tap,id=net0",
        "script=/usr/libexec/qemu-server/pve-bridge",
        "downscript=/usr/libexec/qemu-server/pve-bridgedown",
        "vhost=on",
        "vfio-pci,host=0000:03:00.0,id=hostpci0.0",
        "multifunction=on",
        "vfio-pci,host=0000:03:00.1,id=hostpci0.1,bus=ich9-pcie-port-1,addr=0x0.1",
        "-spice port=5903,addr=0.0.0.0,disable-ticketing=on",
        "virtio-mouse",
        "virtio-keyboard",
        "ivshmem-plain,memdev=ivshmem0,bus=pcie.0",
        "memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M",
        "-audiodev spice,id=spice-backend0",
        "ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc",
        "hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0",
        "hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0",
        "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0",
    ];

    for fragment in shared_fragments {
        assert!(
            proxmox_cmd.contains(fragment),
            "fixture command missing expected fragment: {fragment}"
        );
        assert!(
            generated.contains(fragment),
            "generated command missing expected fragment: {fragment}\n{generated}"
        );
    }

    let generated_only_fragments = ["virtio-serial-pci,id=virtio-serial0,bus=pci.0,addr=0x8"];
    for fragment in generated_only_fragments {
        assert!(
            generated.contains(fragment),
            "generated command missing expected fragment: {fragment}\n{generated}"
        );
    }
}
