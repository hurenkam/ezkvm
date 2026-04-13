use super::QemuArgs;

#[test]
fn test_tpm_uses_server_mode_for_internal_emulator() {
    let tpm_sock = std::env::temp_dir().join("ezkvm-test-tpm.sock");
    let tpm_sock_str = tpm_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    let result = args.add_tpm("2.0", "emulator", &tpm_sock_str, "tpm-tis", false);
    assert!(result.is_ok());

    let built = args.build();
    assert_eq!(built[0], "-chardev");
    assert!(built[1].contains("socket,id=tpmchar,server=on,wait=off,path="));
    assert!(built[1].contains("ezkvm-test-tpm.sock"));
}

#[test]
fn test_tpm_omits_wait_for_external_swtpm_client_mode() {
    let tpm_sock = std::env::temp_dir().join("ezkvm-test-external-tpm.sock");
    let tpm_sock_str = tpm_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    let result = args.add_tpm("2.0", "emulator", &tpm_sock_str, "tpm-tis", true);
    assert!(result.is_ok());

    let built = args.build();
    assert_eq!(built[0], "-chardev");
    assert!(built[1].contains("socket,id=tpmchar,path="));
    assert!(built[1].contains("ezkvm-test-external-tpm.sock"));
    assert!(!built[1].contains("server=on"));
    assert!(!built[1].contains("wait=off"));
}

#[test]
fn test_tpm_rejects_unsupported_backend() {
    let tpm_sock = std::env::temp_dir().join("ezkvm-test-invalid-tpm.sock");
    let tpm_sock_str = tpm_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    let result = args.add_tpm(
        "2.0",
        "unsupported-backend",
        &tpm_sock_str,
        "tpm-tis",
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unsupported TPM backend"));
}

#[test]
fn test_qmp_uses_listening_unix_socket() {
    let qmp_sock = std::env::temp_dir().join("ezkvm-test-qmp.sock");
    let qmp_sock_str = qmp_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    args.add_qmp(Some(&qmp_sock_str), "unix");

    let built = args.build();
    assert_eq!(built[0], "-qmp");
    assert!(built[1].starts_with("unix:"));
    assert!(built[1].contains("ezkvm-test-qmp.sock"));
    assert!(built[1].contains("server=on"));
    assert!(built[1].contains("wait=off"));
}

#[test]
fn test_iscsi_disk_with_initiator_and_auth() {
    let mut args = QemuArgs::new();
    args.add_iscsi_disk(
        "iscsi0",
        "10.0.0.1:3260",
        "iqn.2024-01.example:storage.vm0",
        1,
        Some("iqn.1993-08.org.debian:01:622fd71731a1"),
        Some("chap-user"),
        Some("chap-pass"),
        Some("scsihw0"),
    );

    let built = args.build();
    assert_eq!(built[0], "-blockdev");
    assert!(built[1].contains("driver=iscsi,portal=10.0.0.1:3260,target=iqn.2024-01.example:storage.vm0,lun=1,node-name=iscsi0"));
    assert!(built[1].contains(",initiator-name=iqn.1993-08.org.debian:01:622fd71731a1"));
    assert!(built[1].contains(",user=chap-user"));
    assert!(built[1].contains(",password=chap-pass"));
    assert_eq!(built[2], "-device");
    assert!(built[3].contains("scsi-hd,drive=iscsi0,id=iscsi0,bus=scsihw0.0"));
}
