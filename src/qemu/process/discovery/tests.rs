use super::*;

#[test]
fn test_extract_vm_name_from_cmd() {
    let cmd = "qemu-system-x86_64 -name test-vm -drive file=/var/lib/ezkvm/ubuntu-22.04.qcow2,if=virtio,format=qcow2";
    assert_eq!(extract_vm_name_from_cmd(cmd), Some("test-vm".to_string()));

    let cmd2 = "qemu-system-x86_64 -machine type=q35";
    assert_eq!(extract_vm_name_from_cmd(cmd2), None);
}

#[test]
fn test_contains_exact_qemu_name_arg_exact_match() {
    // Exact match with space after
    assert!(contains_exact_qemu_name_arg(" -name test-vm ", "test-vm"));

    // Exact match at end of command
    assert!(contains_exact_qemu_name_arg(" -name test-vm", "test-vm"));

    // Exact match with quote
    assert!(contains_exact_qemu_name_arg(" -name test-vm\"", "test-vm"));
}

#[test]
fn test_contains_exact_qemu_name_arg_rejects_partial_match() {
    // Should not match partial overlaps
    assert!(!contains_exact_qemu_name_arg(
        " -name test-vm-production ",
        "test-vm"
    ));
    assert!(!contains_exact_qemu_name_arg(
        " -name test-vm-backup ",
        "test-vm"
    ));

    // Should not match if name is substring but not exact
    assert!(!contains_exact_qemu_name_arg(
        " -name prefix-test-vm ",
        "test-vm"
    ));
}

#[test]
fn test_contains_exact_qemu_name_arg_overlapping_names() {
    // Two similar names should not cross-match
    let cmd_for_prod = " -name production-vm ";
    let cmd_for_test = " -name test-vm ";

    assert!(contains_exact_qemu_name_arg(cmd_for_prod, "production-vm"));
    assert!(!contains_exact_qemu_name_arg(cmd_for_prod, "test-vm"));

    assert!(contains_exact_qemu_name_arg(cmd_for_test, "test-vm"));
    assert!(!contains_exact_qemu_name_arg(cmd_for_test, "production-vm"));
}

#[test]
fn test_contains_exact_qemu_name_arg_unsafe_characters() {
    // VM names with underscores, numbers, dashes should work
    assert!(contains_exact_qemu_name_arg(
        " -name my_vm_2024 ",
        "my_vm_2024"
    ));
    assert!(contains_exact_qemu_name_arg(" -name vm-01 ", "vm-01"));

    // Should not match if unsafe chars differ
    assert!(!contains_exact_qemu_name_arg(
        " -name my_vm_2024 ",
        "my-vm-2024"
    ));
}

#[test]
fn test_full_command_line_with_multiple_spaces() {
    let cmd = "qemu-system-x86_64 -machine type=q35 -name test-vm -smp 4 -m 4096";
    assert!(contains_exact_qemu_name_arg(cmd, "test-vm"));
    assert!(!contains_exact_qemu_name_arg(cmd, "test"));
    assert!(!contains_exact_qemu_name_arg(cmd, "vm"));
}
