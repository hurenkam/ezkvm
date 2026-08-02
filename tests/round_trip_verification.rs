//! Phase 9 Plan 09-01: end-to-end Proxmox -> Runtime -> ezkvm YAML -> Runtime -> QEMU cmdline
//! round-trip verification for real corpus fixtures per QEMU-04.

use ezkvm::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
use ezkvm::config::qemu::{QemuCommandLine, QemuContext};
use ezkvm::config::EzkvmConfigSchema;
use ezkvm::runtime::{RootDeviceKind, Runtime};
use std::str::FromStr;

fn assert_precedes(haystack: &str, needle_before: &str, needle_after: &str) {
    let before_pos = haystack
        .find(needle_before)
        .unwrap_or_else(|| panic!("'{}' not found in output", needle_before));
    let after_pos = haystack
        .find(needle_after)
        .unwrap_or_else(|| panic!("'{}' not found in output", needle_after));
    assert!(
        before_pos < after_pos,
        "expected '{}' (pos {}) to precede '{}' (pos {})",
        needle_before,
        before_pos,
        needle_after,
        after_pos
    );
}

fn load_corpus(dir: &str, conf_file: &str) -> (ProxmoxVmConf, ProxmoxStorageConf) {
    let conf_path = format!("input/{dir}/{conf_file}");
    let storage_path = format!("input/{dir}/storage.cfg");

    let conf_str = std::fs::read_to_string(&conf_path)
        .unwrap_or_else(|_| panic!("{conf_path} not found"));
    let storage_str = std::fs::read_to_string(&storage_path)
        .unwrap_or_else(|_| panic!("{storage_path} not found"));
    let vm_conf = ProxmoxVmConf::from_str(&conf_str)
        .unwrap_or_else(|_| panic!("failed to parse {conf_path}"));
    let storage_conf = ProxmoxStorageConf::from_str(&storage_str)
        .unwrap_or_else(|_| panic!("failed to parse {storage_path}"));

    (vm_conf, storage_conf)
}

fn runtime_for_cmdline(full_runtime: Runtime) -> Runtime {
    let mut runtime = Runtime::new();
    for device in full_runtime.root_devices() {
        if device.device_kind() != RootDeviceKind::EfiDisk {
            runtime.register_root_device(device.clone());
        }
    }
    runtime
}

fn make_ctx(vm_name: &str) -> QemuContext {
    QemuContext::new(
        vm_name.to_string(),
        "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
        Some(format!("/tmp/{vm_name}-tpm.sock")),
    )
}

#[test]
fn felucia_108_full_round_trip_produces_valid_qemu_cmdline() {
    let (vm_conf, storage_conf) = load_corpus("felucia", "108.conf");
    let rawargs_substring = vm_conf
        .args
        .clone()
        .expect("108.conf active section must have an args line");

    let original_runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");

    let schema = EzkvmConfigSchema::try_from(original_runtime)
        .expect("runtime -> ezkvm yaml schema conversion failed");
    let yaml = schema
        .to_styled_compact_yaml()
        .expect("ezkvm yaml serialization failed");
    assert!(!yaml.is_empty(), "serialized ezkvm YAML must not be empty");
    let parsed_schema =
        EzkvmConfigSchema::from_str(&yaml).expect("ezkvm yaml schema parsing failed");
    let round_tripped_runtime =
        Runtime::try_from(parsed_schema).expect("ezkvm yaml schema -> runtime conversion failed");

    let runtime = runtime_for_cmdline(round_tripped_runtime);
    let cmdline =
        QemuCommandLine::try_from((runtime, make_ctx("felucia"))).expect("qemu cmdline try_from");
    let output = cmdline.to_string();

    assert_precedes(&output, "id=drive-scsi0", "drive=drive-scsi0");
    assert_precedes(&output, "id=drive-ide2", "drive=drive-ide2");

    assert_eq!(
        output.matches(&rawargs_substring).count(),
        1,
        "expected RawArgs blob to appear exactly once in output: {output}"
    );
    let rawargs_pos = output
        .find(&rawargs_substring)
        .expect("RawArgs blob missing from output");
    let last_real_device = cmdline
        .devices()
        .last()
        .expect("devices segment must be non-empty for the felucia fixture")
        .clone();
    let last_device_pos = output[..rawargs_pos].rfind(&last_real_device).unwrap_or_else(|| {
        panic!(
            "expected devices segment's last token '{}' to appear before the RawArgs blob in output: {output}",
            last_real_device
        )
    });
    assert!(
        rawargs_pos > last_device_pos,
        "expected RawArgs blob (pos {rawargs_pos}) to be positioned after every other segment's last token (last device at pos {last_device_pos}); output was: {output}"
    );

    assert_eq!(
        output.matches("vfio-pci").count(),
        2,
        "expected exactly 2 vfio-pci devices in output: {output}"
    );
    assert_eq!(
        output.matches("usb-host").count(),
        1,
        "expected exactly 1 usb-host device in output: {output}"
    );
    assert!(output.contains("hostbus=1"), "output was: {output}");
    assert!(output.contains("hostport=2.2"), "output was: {output}");

    assert_eq!(
        output.matches("id=drive-scsi0").count(),
        1,
        "expected exactly 1 drive-scsi0 definition in output: {output}"
    );
    assert_eq!(
        output.matches("id=drive-scsi1").count(),
        1,
        "expected exactly 1 drive-scsi1 definition in output: {output}"
    );
    assert_eq!(
        output.matches("drive=drive-scsi0").count(),
        1,
        "expected exactly 1 drive-scsi0 device reference in output: {output}"
    );
    assert_eq!(
        output.matches("drive=drive-scsi1").count(),
        1,
        "expected exactly 1 drive-scsi1 device reference in output: {output}"
    );
    assert_eq!(
        output.matches("-device tpm-tis,tpmdev=tpmdev").count(),
        1,
        "expected exactly 1 TPM device entry in output: {output}"
    );
    assert_eq!(
        output
            .matches("-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5")
            .count(),
        1,
        "expected exactly 1 pvscsi controller entry in output: {output}"
    );
}


#[test]
fn coruscant_501_full_round_trip_produces_valid_qemu_cmdline() {
    let (vm_conf, storage_conf) = load_corpus("coruscant", "501.conf");
    let rawargs_substring = vm_conf
        .args
        .clone()
        .expect("501.conf active section must have an args line");

    let original_runtime = ProxmoxImporter::new(vm_conf, storage_conf, 501)
        .into_runtime()
        .expect("into_runtime");

    let schema = EzkvmConfigSchema::try_from(original_runtime)
        .expect("runtime -> ezkvm yaml schema conversion failed");
    let yaml = schema
        .to_styled_compact_yaml()
        .expect("ezkvm yaml serialization failed");
    assert!(!yaml.is_empty(), "serialized ezkvm YAML must not be empty");
    let parsed_schema =
        EzkvmConfigSchema::from_str(&yaml).expect("ezkvm yaml schema parsing failed");
    let round_tripped_runtime =
        Runtime::try_from(parsed_schema).expect("ezkvm yaml schema -> runtime conversion failed");

    let runtime = runtime_for_cmdline(round_tripped_runtime);
    let cmdline =
        QemuCommandLine::try_from((runtime, make_ctx("coruscant"))).expect("qemu cmdline try_from");
    let output = cmdline.to_string();

    assert_eq!(
        output.matches("vfio-pci").count(),
        3,
        "expected exactly 3 vfio-pci devices in output: {output}"
    );
    assert_eq!(
        output
            .matches("-device virtio-scsi-pci,id=scsihw0,bus=pci.0,addr=0x5")
            .count(),
        1,
        "expected exactly 1 virtio-scsi-pci controller entry in output: {output}"
    );

    for scsi_id in 0..=2 {
        let drive_id = format!("id=drive-scsi{scsi_id}");
        let device_drive = format!("drive=drive-scsi{scsi_id}");
        assert_eq!(
            output.matches(&drive_id).count(),
            1,
            "expected exactly 1 {drive_id} definition in output: {output}"
        );
        assert_eq!(
            output.matches(&device_drive).count(),
            1,
            "expected exactly 1 {device_drive} device reference in output: {output}"
        );
        assert_precedes(&output, &drive_id, &device_drive);
    }
    assert_eq!(
        output.matches("bus=scsihw0.0").count(),
        3,
        "expected exactly 3 shared-bus scsi device attachments in output: {output}"
    );

    assert_eq!(
        output.matches(&rawargs_substring).count(),
        1,
        "expected RawArgs blob to appear exactly once in output: {output}"
    );
    let rawargs_pos = output
        .find(&rawargs_substring)
        .expect("RawArgs blob missing from output");
    let last_real_device = cmdline
        .devices()
        .last()
        .expect("devices segment must be non-empty for the coruscant fixture")
        .clone();
    let last_device_pos = output[..rawargs_pos].rfind(&last_real_device).unwrap_or_else(|| {
        panic!(
            "expected devices segment's last token '{}' to appear before the RawArgs blob in output: {output}",
            last_real_device
        )
    });
    assert!(
        rawargs_pos > last_device_pos,
        "expected RawArgs blob (pos {rawargs_pos}) to be positioned after every other segment's last token (last device at pos {last_device_pos}); output was: {output}"
    );

    assert!(
        !output.contains("netdev=net0"),
        "did not expect fabricated net0 in output: {output}"
    );
    assert!(
        !output.contains("virtio-net-pci"),
        "did not expect fabricated virtio-net-pci device in output: {output}"
    );
}

#[test]
fn zbp_server_mh2_301_full_round_trip_produces_valid_qemu_cmdline() {
    let (vm_conf, storage_conf) = load_corpus("zbp-server-mh2", "301.conf");

    let original_runtime = ProxmoxImporter::new(vm_conf, storage_conf, 301)
        .into_runtime()
        .expect("into_runtime");

    let schema = EzkvmConfigSchema::try_from(original_runtime)
        .expect("runtime -> ezkvm yaml schema conversion failed");
    let yaml = schema
        .to_styled_compact_yaml()
        .expect("ezkvm yaml serialization failed");
    assert!(!yaml.is_empty(), "serialized ezkvm YAML must not be empty");
    let parsed_schema =
        EzkvmConfigSchema::from_str(&yaml).expect("ezkvm yaml schema parsing failed");
    let round_tripped_runtime =
        Runtime::try_from(parsed_schema).expect("ezkvm yaml schema -> runtime conversion failed");

    let runtime = runtime_for_cmdline(round_tripped_runtime);
    let cmdline = QemuCommandLine::try_from((runtime, make_ctx("zbp-server-mh2")))
        .expect("qemu cmdline try_from");
    let output = cmdline.to_string();

    assert_eq!(
        output.matches("usb-host").count(),
        6,
        "expected exactly 6 usb-host devices in output: {output}"
    );
    assert_eq!(
        output.matches("hostbus=").count(),
        4,
        "expected exactly 4 bus-port usb passthrough entries in output: {output}"
    );
    assert!(output.contains("hostport=4"), "output was: {output}");
    assert!(output.contains("hostport=5"), "output was: {output}");
    assert!(output.contains("hostport=7.6"), "output was: {output}");
    assert!(output.contains("hostport=7.5.1"), "output was: {output}");
    assert!(output.contains("vendorid=0x0451"), "output was: {output}");
    assert!(output.contains("productid=0x16a0"), "output was: {output}");
    assert!(output.contains("vendorid=0x0403"), "output was: {output}");
    assert!(output.contains("productid=0x6001"), "output was: {output}");

    assert_eq!(
        output.matches("vfio-pci").count(),
        1,
        "expected exactly 1 vfio-pci device in output: {output}"
    );
    assert!(output.contains("virtio-net-pci"), "output was: {output}");
    assert!(output.contains("id=net20"), "output was: {output}");
    assert!(output.contains("netdev=net20"), "output was: {output}");
    assert_precedes(&output, "id=net20", "netdev=net20");

    assert_eq!(
        output.matches("-device virtio-scsi-pci,id=scsihw").count(),
        4,
        "expected exactly 4 virtio-scsi-single controller entries in output: {output}"
    );
    for controller_id in 0..=3 {
        let iothread = format!("-object iothread,id=iothread{controller_id}");
        let controller = format!("-device virtio-scsi-pci,id=scsihw{controller_id}");
        assert_eq!(
            output.matches(&iothread).count(),
            1,
            "expected exactly 1 {iothread} object in output: {output}"
        );
        assert_eq!(
            output.matches(&controller).count(),
            1,
            "expected exactly 1 {controller} device in output: {output}"
        );
        assert_precedes(&output, &iothread, &controller);
    }

    assert_precedes(&output, "id=drive-scsi0", "drive=drive-scsi0");
    assert_precedes(&output, "id=drive-scsi1-0", "drive=drive-scsi1-0");
    assert_precedes(&output, "id=drive-scsi2-0", "drive=drive-scsi2-0");
    assert_precedes(&output, "id=drive-scsi3-0", "drive=drive-scsi3-0");
    assert!(output.contains("id=scsi0"), "output was: {output}");
    assert!(output.contains("id=scsi1-0"), "output was: {output}");
    assert!(output.contains("id=scsi2-0"), "output was: {output}");
    assert!(output.contains("id=scsi3-0"), "output was: {output}");
}
