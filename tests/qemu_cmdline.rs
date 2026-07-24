use ezkvm::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
use ezkvm::config::qemu::{QemuCommandLine, QemuContext};
use ezkvm::runtime::{
    Chipset, Memory, PcieAddress, Q35ChipsetBuilder, Runtime, RuntimeBuilder, RootDeviceKind,
    VirtioNetPcie,
};
use std::str::FromStr;
use std::sync::Arc;

/// Asserts that `needle_before`'s first occurrence in `haystack` is at a strictly lower
/// byte index than `needle_after`'s first occurrence. Panics naming both needles and their
/// positions (or their absence) when the order is violated — reused across every
/// drive/device and netdev/device ordering assertion in this file (07-VALIDATION.md Wave 0
/// requirement).
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

/// Copy of `tests/proxmox_import.rs`'s `load_felucia_108()` shape — deliberately not shared
/// via `use` since Rust integration test binaries cannot import each other's code without a
/// shared test-support module, which is out of scope for this plan (see PLAN.md Task 1 action).
fn load_felucia_108() -> (ProxmoxVmConf, ProxmoxStorageConf) {
    let conf_str = std::fs::read_to_string("input/felucia/108.conf")
        .expect("input/felucia/108.conf not found");
    let storage_str = std::fs::read_to_string("input/felucia/storage.cfg")
        .expect("input/felucia/storage.cfg not found");
    let vm_conf = ProxmoxVmConf::from_str(&conf_str).expect("parse 108.conf");
    let storage_conf = ProxmoxStorageConf::from_str(&storage_str).expect("parse storage.cfg");
    (vm_conf, storage_conf)
}

fn make_ctx() -> QemuContext {
    QemuContext::new(
        "felucia".to_string(),
        "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
        Some("/tmp/felucia-tpm.sock".to_string()),
    )
}

/// Re-registers every root device from a fully-imported felucia `Runtime` EXCEPT `EfiDisk`
/// into a fresh `Runtime`, dropping `boot_order` in the process (there is no public setter
/// on an already-built `Runtime` other than `RuntimeBuilder::with_boot_order`, which would
/// require re-downcasting every device kind for no benefit to this file's assertions — see
/// Deviations in the SUMMARY).
///
/// This routes around a pre-existing, already-documented (Plan 07-04 "Known Stubs") gap:
/// `ProxmoxImporter` never resolves `EfiDisk.block_device_size_bytes` for imported configs
/// (confirmed by `src/config/proxmox/importer.rs`'s own
/// `test_04_02_efidisk_logical_size_from_options`, which asserts it is `None`), and
/// `emit_root_device`'s `EfiDisk` arm `.expect()`-panics without it (`D-06`). Plan 07-04
/// worked around the identical panic in `root.rs`'s own test module via
/// `felucia_chipset_and_boot_order()`; this helper generalizes that pattern so the
/// RawArgs/AudioDevice/etc. root devices this plan's tests need are preserved too, not just
/// the `Chipset` + `boot_order` that Plan 07-04 needed for its bootindex-only assertions.
/// The ordering truths this plan proves (drive-before-device, RawArgs-at-end) do not depend
/// on `boot_order`/`bootindex=` values, so dropping `boot_order` here does not weaken them.
fn felucia_runtime_for_cmdline() -> Runtime {
    let (vm_conf, storage_conf) = load_felucia_108();
    let full_runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");

    let mut runtime = Runtime::new();
    for device in full_runtime.root_devices() {
        if device.device_kind() != RootDeviceKind::EfiDisk {
            runtime.register_root_device(device.clone());
        }
    }
    runtime
}

#[test]
fn test_assert_precedes_panics_naming_both_needles_and_positions() {
    let result = std::panic::catch_unwind(|| {
        assert_precedes("id=drive-scsi0 drive=drive-scsi0", "drive=drive-scsi0", "id=drive-scsi0");
    });
    let err = result.expect_err("assert_precedes should panic on violated order");
    let msg = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .expect("panic payload should be a string");
    assert!(msg.contains("drive=drive-scsi0"), "panic message was: {msg}");
    assert!(msg.contains("id=drive-scsi0"), "panic message was: {msg}");
    assert!(msg.contains("pos"), "panic message was: {msg}");
}

#[test]
fn test_assert_precedes_passes_when_order_correct() {
    // Should not panic.
    assert_precedes("id=drive-scsi0 drive=drive-scsi0", "id=drive-scsi0", "drive=drive-scsi0");
}

#[test]
fn test_qemu_cmdline_felucia_108_drive_before_device_ordering() {
    let runtime = felucia_runtime_for_cmdline();
    let cmdline = QemuCommandLine::try_from((runtime, make_ctx())).expect("try_from");
    let output = cmdline.to_string();

    // scsi0 — the fixture's boot disk (vm1-pool:vm-108-boot)
    assert_precedes(&output, "id=drive-scsi0", "drive=drive-scsi0");
    // ide2 — the fixture's empty CD-ROM (ide2: none,media=cdrom), imported per Plan 07-04's
    // documented deviation as a Cdrom("none") at IdeAddress(channel=1, device=0) -> label ide2
    assert_precedes(&output, "id=drive-ide2", "drive=drive-ide2");
}

#[test]
fn test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end() {
    // Capture the raw args string directly from the parsed conf (D-06: args is opaque,
    // never parsed, only compared verbatim here) before building the Runtime used for
    // conversion.
    let (vm_conf, _) = load_felucia_108();
    let rawargs_substring = vm_conf
        .args
        .clone()
        .expect("108.conf's active section must have an args: line");

    let runtime = felucia_runtime_for_cmdline();
    let cmdline = QemuCommandLine::try_from((runtime, make_ctx())).expect("try_from");

    // The plan's last-emitted "-device" must be resolved from the actual devices segment,
    // not via a generic `rfind("-device")` over the full rendered output — the RawArgs blob
    // itself contains multiple embedded "-device ..." substrings (e.g.
    // "-device virtio-serial-pci", "-device virtio-mouse"), so a naive whole-output
    // `rfind("-device")` would match *inside* the verbatim blob rather than the real
    // qemu-emitted devices segment, which is a false-positive that would make this
    // assertion pass or fail for the wrong reason. See Deviations in the SUMMARY.
    let last_real_device = cmdline
        .devices()
        .last()
        .expect("devices segment must be non-empty for the felucia fixture")
        .clone();

    let output = cmdline.to_string();

    // RawArgs appears exactly once, as one contiguous unbroken substring (QEMU-03).
    assert_eq!(
        output.matches(&rawargs_substring).count(),
        1,
        "expected RawArgs blob to appear exactly once in output: {output}"
    );

    let rawargs_pos = output
        .find(&rawargs_substring)
        .expect("RawArgs blob missing from output");

    // Search for the last real device token strictly *before* where the RawArgs blob
    // starts, so a coincidental occurrence of the same text inside the blob (there is
    // none for this fixture, but this keeps the assertion robust) can't produce a false
    // pass.
    let last_device_pos = output[..rawargs_pos]
        .rfind(&last_real_device)
        .unwrap_or_else(|| {
            panic!(
                "expected devices segment's last token '{}' to appear before the RawArgs blob in output: {output}",
                last_real_device
            )
        });

    assert!(
        rawargs_pos > last_device_pos,
        "expected RawArgs blob (pos {rawargs_pos}) to be positioned after every other \
         segment's last token (last device at pos {last_device_pos}); output was: {output}"
    );
}

#[test]
fn test_qemu_cmdline_synthetic_netdev_before_device_ordering() {
    let runtime: Runtime = RuntimeBuilder::new()
        .with_memory(Memory::new(4096))
        .with_chipset(Chipset::Q35(
            Q35ChipsetBuilder::new()
                .with_pcie_device(
                    Some(PcieAddress::new(20, 0)),
                    Arc::new(VirtioNetPcie::new(
                        Some("vmbr0".to_string()),
                        Some("AA:BB:CC:DD:EE:FF".to_string()),
                        Some(1024),
                        Some(256),
                        Some(true),
                    )),
                )
                .build(),
        ))
        .build()
        .expect("build synthetic runtime");

    let ctx = QemuContext::new(
        "synthetic".to_string(),
        "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
        None,
    );

    let cmdline = QemuCommandLine::try_from((runtime, ctx)).expect("try_from");
    let output = cmdline.to_string();

    // RESEARCH.md Pitfall 7: the felucia fixture has only one NIC and cannot alone prove
    // netdev-before-device ordering in general; this synthetic Runtime, hand-constructed
    // at PcieAddress(20, 0), closes that gap.
    assert_precedes(&output, "id=net20", "netdev=net20");

    // Regression guard: an empty boot_order must never emit a bootindex= token anywhere,
    // confirmed at the test-file level (not just handlers/root.rs's own unit tests).
    assert!(
        !output.contains("bootindex="),
        "expected no bootindex= token for empty boot_order; output was: {output}"
    );
}
