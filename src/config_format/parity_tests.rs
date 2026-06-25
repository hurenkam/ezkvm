//! Cross-format parity tests for shared ezkvm <-> Proxmox supported subsets.
//!
//! These tests intentionally focus only on fields currently supported by both adapters.

use crate::config_format::{
    ExportOptions, ImportOptions,
    proxmox::{ProxmoxConfigSchema, storage_resolver::ProxmoxStorageConfig},
};
use crate::runtime_model::RuntimeModel;

fn temp_path(tag: &str, ext: &str) -> String {
    format!("/tmp/ezkvm-parity-{}-{}.{}", std::process::id(), tag, ext)
}

fn proxmox_storage_cfg_text() -> &'static str {
    r#"
dir: local
    path /var/lib/vz

lvmthin: vm-pool
    vgname vm

lvmthin: vm1-pool
    vgname vm1
"#
}

fn write_storage_cfg(tag: &str) -> String {
    let path = temp_path(&format!("storage-{tag}"), "cfg");
    std::fs::write(&path, proxmox_storage_cfg_text()).expect("storage.cfg should be written");
    path
}

fn import_proxmox_to_runtime(conf_text: &str, tag: &str) -> RuntimeModel {
    let vm_path = temp_path(&format!("in-{tag}"), "conf");
    let storage_path = write_storage_cfg(tag);
    std::fs::write(&vm_path, conf_text).expect("proxmox conf should be written");

    ImportOptions::Proxmox {
        storage: storage_path,
        vm: vm_path,
    }
    .import_runtime()
    .expect("proxmox import should succeed")
}

fn export_runtime_to_ezkvm(runtime: RuntimeModel, tag: &str) -> String {
    let host_path = temp_path(&format!("host-{tag}"), "yaml");
    let vm_path = temp_path(&format!("out-{tag}"), "yaml");

    std::fs::write(&host_path, "name: host\n").expect("host file should be written");

    let output_path = ExportOptions::Ezkvm {
        host: host_path,
        vm: Some(vm_path.clone()),
    }
    .export_runtime(runtime)
    .expect("ezkvm export should succeed");

    std::fs::read_to_string(output_path).expect("ezkvm output should be readable")
}

fn import_ezkvm_to_runtime(yaml_text: &str, tag: &str) -> RuntimeModel {
    let host_path = temp_path(&format!("host-{tag}"), "yaml");
    let parsed: serde_json::Value =
        serde_yaml::from_str(yaml_text).expect("ezkvm yaml should parse for vm_name extraction");
    let vm_name = parsed
        .get("metadata")
        .and_then(|m| m.get("vm_name"))
        .and_then(|v| v.as_str())
        .expect("metadata.vm_name must exist in parity fixture");
    let vm_path = format!("/tmp/{vm_name}.yaml");

    std::fs::write(&host_path, "name: host\n").expect("host file should be written");
    std::fs::write(&vm_path, yaml_text).expect("ezkvm yaml should be written");

    ImportOptions::Ezkvm {
        host: host_path,
        vm: vm_path,
    }
    .import_runtime()
    .expect("ezkvm import should succeed")
}

fn export_runtime_to_proxmox(runtime: RuntimeModel, tag: &str) -> String {
    let storage_path = write_storage_cfg(tag);
    let vm_path = temp_path(&format!("out-{tag}"), "conf");

    let output_path = ExportOptions::Proxmox {
        storage: storage_path,
        vm: vm_path,
    }
    .export_runtime(runtime)
    .expect("proxmox export should succeed");

    std::fs::read_to_string(output_path).expect("proxmox output should be readable")
}

fn parse_proxmox_entries(text: &str) -> std::collections::BTreeMap<String, String> {
    let schema = ProxmoxConfigSchema::parse(text).expect("proxmox conf should parse");
    let mut out = std::collections::BTreeMap::new();

    for key in schema.global.entries.keys() {
        let line = text
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with(&format!("{key}:")))
            .unwrap_or_default();

        let value = line
            .split_once(':')
            .map(|(_, rhs)| rhs.trim().to_string())
            .unwrap_or_default();

        out.insert(key.clone(), value);
    }

    out
}

#[test]
fn proxmox_to_ezkvm_shared_subset_parity() {
    let proxmox_source = r#"
name: parity-vm
machine: q35
memory: 4096
cpu: host
cores: 4
sockets: 1
bios: ovmf
efidisk0: /var/lib/vz/images/parity/efivars.fd
scsi0: vm-pool:vm-100-disk-0,cache=writeback,size=32G
net0: virtio=DE:AD:BE:EF:00:42,bridge=vmbr0
"#;

    let runtime = import_proxmox_to_runtime(proxmox_source, "p2e");
    let ezkvm_yaml = export_runtime_to_ezkvm(runtime, "p2e");

    let value: serde_json::Value =
        serde_yaml::from_str(&ezkvm_yaml).expect("ezkvm yaml should parse as value");

    assert_eq!(value["metadata"]["vm_name"], "parity-vm");
    assert_eq!(value["virtual_machine"]["machine"]["chipset"], "q35");
    assert_eq!(
        value["virtual_machine"]["memory"]["size"],
        4096u64 * 1024 * 1024
    );
    assert_eq!(value["virtual_machine"]["cpu"]["cores"], 4);
    assert_eq!(value["virtual_machine"]["cpu"]["sockets"], 1);
    assert!(value["virtual_machine"]["boot"].get("uefi").is_some());

    let resources = value["host"]["resources"]
        .as_array()
        .expect("host.resources should be an array");
    assert!(resources.iter().any(|res| {
        res.get("storage")
            .and_then(|s| s.get("block_device"))
            .and_then(|v| v.as_str())
            == Some("/dev/vm/vm-100-disk-0")
    }));
    assert!(resources.iter().any(|res| {
        res.get("network")
            .and_then(|n| n.get("bridge"))
            .and_then(|v| v.as_str())
            == Some("vmbr0")
    }));
}

#[test]
fn proxmox_hostpci_maps_to_host_resource_reference() {
    let proxmox_source = r#"
name: hostpci-parity-vm
machine: q35
memory: 4096
cpu: host
cores: 2
sockets: 1
hostpci0: 0000:0e:11.6,pcie=1,rombar=0
"#;

    let runtime = import_proxmox_to_runtime(proxmox_source, "p2e-hostpci");
    let ezkvm_yaml = export_runtime_to_ezkvm(runtime, "p2e-hostpci");

    let value: serde_json::Value =
        serde_yaml::from_str(&ezkvm_yaml).expect("ezkvm yaml should parse as value");

    let resources = value["host"]["resources"]
        .as_array()
        .expect("host.resources should be an array");
    assert!(resources.iter().any(|res| {
        res.get("id").and_then(|v| v.as_str()) == Some("hostpci0")
            && res
                .get("pcie")
                .and_then(|p| p.get("address"))
                .and_then(|v| v.as_str())
                == Some("0000:0e:11.6")
            && res
                .get("pcie")
                .and_then(|p| p.get("rombar"))
                .and_then(|v| v.as_bool())
                == Some(false)
    }));

    let devices = value["virtual_machine"]["devices"]
        .as_array()
        .expect("virtual_machine.devices should be an array");
    assert!(devices.iter().any(|device| {
        device
            .get("pcie")
            .and_then(|p| p.get("type"))
            .and_then(|v| v.as_str())
            == Some("passthrough")
            && device
                .get("pcie")
                .and_then(|p| p.get("resource"))
                .and_then(|v| v.as_str())
                == Some("hostpci0")
    }));
}

#[test]
fn ezkvm_to_proxmox_shared_subset_parity() {
    let ezkvm_source = r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "parity-e2p"
host:
  resources:
    - id: "fw0"
      storage:
        file: "/var/lib/vz/images/parity/efivars.fd"
    - id: "disk0"
      storage:
        block_device: "/dev/vm/vm-200-disk-0"
    - id: "net0"
      network:
        bridge: "vmbr2"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  cpu:
    model: Host
    cores: 2
    threads: 1
    sockets: 1
  memory:
    size: 8589934592
  boot:
    uefi:
      resource: "fw0"
  devices:
    - pcie:
        type: "pv_scsi"
    - scsi:
        type: "hdd"
        resource: "disk0"
    - pcie:
        type: "virtio_net"
        resource: "net0"
"#;

    let runtime = import_ezkvm_to_runtime(ezkvm_source, "e2p");
    let proxmox_conf = export_runtime_to_proxmox(runtime, "e2p");
    let entries = parse_proxmox_entries(&proxmox_conf);

    assert_eq!(entries.get("name").map(String::as_str), Some("parity-e2p"));
    assert_eq!(entries.get("machine").map(String::as_str), Some("q35"));
    assert_eq!(entries.get("memory").map(String::as_str), Some("8192"));
    assert_eq!(entries.get("cpu").map(String::as_str), Some("host"));
    assert_eq!(entries.get("cores").map(String::as_str), Some("2"));
    assert_eq!(entries.get("sockets").map(String::as_str), Some("1"));
    assert_eq!(entries.get("bios").map(String::as_str), Some("ovmf"));

    let efidisk0 = entries
        .get("efidisk0")
        .expect("efidisk0 should be emitted for UEFI");
    assert!(efidisk0.contains("local:"));

    let scsi0 = entries.get("scsi0").expect("scsi0 should be present");
    assert!(scsi0.contains("vm-pool:vm-200-disk-0"));

    let net0 = entries.get("net0").expect("net0 should be present");
    assert!(net0.contains("virtio"));
    assert!(net0.contains("bridge=vmbr2"));
}

#[test]
fn proxmox_ezkvm_proxmox_roundtrip_keeps_shared_subset_fields() {
    let proxmox_source = r#"
name: parity-rtrip
machine: q35
memory: 6144
cpu: host
cores: 3
sockets: 1
bios: ovmf
efidisk0: /var/lib/vz/images/parity/efivars.fd
scsi0: vm-pool:vm-300-disk-0,cache=writeback,size=48G
net0: virtio=DE:AD:BE:EF:00:99,bridge=vmbr9
"#;

    let runtime_from_proxmox = import_proxmox_to_runtime(proxmox_source, "rtrip-a");
    let ezkvm_yaml = export_runtime_to_ezkvm(runtime_from_proxmox, "rtrip-a");
    let runtime_from_ezkvm = import_ezkvm_to_runtime(&ezkvm_yaml, "rtrip-b");
    let proxmox_after = export_runtime_to_proxmox(runtime_from_ezkvm, "rtrip-b");

    let before = parse_proxmox_entries(proxmox_source);
    let after = parse_proxmox_entries(&proxmox_after);

    let keys = [
        "name", "machine", "memory", "cpu", "cores", "sockets", "bios",
    ];

    for key in keys {
        assert_eq!(
            after.get(key),
            before.get(key),
            "shared subset key '{key}' should roundtrip"
        );
    }

    let parsed_storage =
        ProxmoxStorageConfig::parse(proxmox_storage_cfg_text()).expect("storage cfg should parse");

    let efidisk_after_token = after
        .get("efidisk0")
        .expect("efidisk0 should still be present");
    let efidisk_after_resource = parsed_storage
        .token_to_resource(efidisk_after_token)
        .expect("efidisk0 after token should map to resource");
    let efidisk_before_token = before
        .get("efidisk0")
        .expect("efidisk0 should be present before");
    let efidisk_before_resource = parsed_storage
        .token_to_resource(efidisk_before_token)
        .expect("efidisk0 before token should map to resource");

    assert_eq!(
        format!("{efidisk_after_resource:?}"),
        format!("{efidisk_before_resource:?}"),
        "efidisk0 resource should roundtrip even if token representation changes"
    );

    let scsi_after_token = after
        .get("scsi0")
        .expect("scsi0 should still be present")
        .split(',')
        .next()
        .expect("scsi0 token should be parseable");
    let scsi_after_resource = parsed_storage
        .token_to_resource(scsi_after_token)
        .expect("scsi0 token should map to resource");
    let scsi_before_token = before
        .get("scsi0")
        .expect("scsi0 should be present before")
        .split(',')
        .next()
        .expect("scsi0 before token should be parseable");
    let scsi_before_resource = parsed_storage
        .token_to_resource(scsi_before_token)
        .expect("scsi0 before token should map to resource");

    assert_eq!(
        std::mem::discriminant(&scsi_after_resource),
        std::mem::discriminant(&scsi_before_resource),
        "scsi storage kind should remain stable"
    );

    let net_after = after.get("net0").expect("net0 should still be present");
    let net_before = before.get("net0").expect("net0 should be present before");
    assert!(
        net_after.contains("virtio") && net_before.contains("virtio"),
        "net0 should remain a virtio network device"
    );
    assert!(
        net_after.contains("bridge=vmbr9") && net_before.contains("bridge=vmbr9"),
        "net0 bridge binding should remain stable"
    );
}
