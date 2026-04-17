// Temporary over-size rationale (B-29): compaction orchestration and tests are still
// in one file after B-27 extraction. Closure target is <=250 lines by moving the
// integration-style test harness to dedicated test modules.
use super::ImportError;
use serde_yaml::{Mapping, Value};

mod compact;
mod loader;
mod merge;
mod paths;
mod yaml;

pub fn compact_profile_owned_fields(yaml: &str) -> Result<String, ImportError> {
    let vm_value: Value = serde_yaml::from_str(yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to parse generated YAML for profile compaction: {e}"
        ))
    })?;

    let profile_names = loader::extract_profile_names(&vm_value)?;
    if profile_names.is_empty() {
        return Ok(yaml.to_string());
    }

    let profile_dir = loader::resolve_profile_dir();
    let base_profiles = loader::load_merged_profiles(&profile_dir, &profile_names)?;

    let compacted_value = compact::compact_overlay_against_base(&base_profiles, &vm_value, &[])
        .unwrap_or_else(|| Value::Mapping(Mapping::new()));

    serde_yaml::to_string(&compacted_value).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to serialize compacted profile-aware YAML: {e}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::compact_profile_owned_fields;
    use crate::config::VmConfig;
    use crate::test_support::env_lock;
    use serde_yaml::Value;
    use std::path::PathBuf;

    fn with_test_profiles<T>(run: impl FnOnce(PathBuf) -> T) -> T {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = std::env::temp_dir().join(format!(
            "ezkvm-profile-compact-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).expect("create profile dir");

        let central = root.join("ezkvm.yaml");
        std::fs::write(
            &central,
            format!("locations:\n  profile_dir: {}\n", profile_dir.display()),
        )
        .expect("write central config");

        let old = std::env::var_os("EZKVM_CONFIG");
        unsafe { std::env::set_var("EZKVM_CONFIG", &central) };

        let result = run(profile_dir);

        unsafe {
            match old {
                Some(value) => std::env::set_var("EZKVM_CONFIG", value),
                None => std::env::remove_var("EZKVM_CONFIG"),
            }
        }

        result
    }

    #[test]
    fn removes_redundant_mapping_fields_owned_by_profiles() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("windows-common.yaml"),
                "options:\n  rtc:\n    base: localtime\n    driftfix: slew\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - windows-common
system:
  architecture: x86_64
options:
  rtc:
    base: localtime
    driftfix: slew
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let map = value.as_mapping().expect("root mapping");
            assert!(map.contains_key(Value::String("profiles".to_string())));
            assert!(map.contains_key(Value::String("system".to_string())));
            assert!(!map.contains_key(Value::String("options".to_string())));
        });
    }

    #[test]
    fn keeps_vm_specific_values_when_they_differ_from_profiles() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("windows-common.yaml"),
                "options:\n  rtc:\n    base: localtime\n    driftfix: slew\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - windows-common
options:
  rtc:
    base: utc
    driftfix: slew
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let rtc_base = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("options".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("rtc".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("base".to_string())))
                .and_then(Value::as_str);

            assert_eq!(rtc_base, Some("utc"));
        });
    }

    #[test]
    fn compacts_id_merge_lists_by_keeping_only_item_differences() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("gpu-passthrough.yaml"),
                "controllers:\n  xhci:\n    - id: xhci\n      p2: 15\n      p3: 15\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - gpu-passthrough
controllers:
  xhci:
    - id: xhci
      p2: 15
      p3: 15
      bus: pci.1
      addr: "0x1b"
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let xhci = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("controllers".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("xhci".to_string())))
                .and_then(Value::as_sequence)
                .expect("xhci sequence should be present");

            let item = xhci
                .first()
                .and_then(Value::as_mapping)
                .expect("xhci item mapping");

            assert_eq!(
                item.get(Value::String("id".to_string()))
                    .and_then(Value::as_str),
                Some("xhci")
            );
            assert_eq!(
                item.get(Value::String("bus".to_string()))
                    .and_then(Value::as_str),
                Some("pci.1")
            );
            assert_eq!(
                item.get(Value::String("addr".to_string()))
                    .and_then(Value::as_str),
                Some("0x1b")
            );
            assert!(!item.contains_key(Value::String("p2".to_string())));
            assert!(!item.contains_key(Value::String("p3".to_string())));
        });
    }

    #[test]
    fn compacts_append_unique_lists_to_only_new_entries() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("windows-common.yaml"),
                "system:\n  cpu:\n    features:\n      - +kvm_pv_eoi\n      - +kvm_pv_unhalt\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - windows-common
system:
  cpu:
    features:
      - +kvm_pv_eoi
      - +kvm_pv_unhalt
      - +svm
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let features = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("system".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("cpu".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("features".to_string())))
                .and_then(Value::as_sequence)
                .expect("features sequence should remain with new entries");

            assert_eq!(features.len(), 1);
            assert_eq!(features[0].as_str(), Some("+svm"));
        });
    }

    #[test]
    fn compacts_hugepages_profile_owned_defaults_but_keeps_vm_specific_fields() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("hugepages.yaml"),
                "system:\n  memory:\n    hugepages:\n      enabled: true\n      prealloc: true\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - hugepages
system:
  memory:
    hugepages:
      enabled: true
      prealloc: true
      size_kib: 1048576
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let hugepages = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("system".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("memory".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("hugepages".to_string())))
                .and_then(Value::as_mapping)
                .expect("hugepages should remain with vm-specific fields");

            assert!(!hugepages.contains_key(Value::String("enabled".to_string())));
            assert!(!hugepages.contains_key(Value::String("prealloc".to_string())));
            assert_eq!(
                hugepages
                    .get(Value::String("size_kib".to_string()))
                    .and_then(Value::as_u64),
                Some(1048576)
            );
        });
    }

    #[test]
    fn compacts_input_devices_append_unique_to_new_entries_only() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("remote-viewer-spice.yaml"),
                "devices:\n  input:\n    - {type: virtio-mouse}\n    - {type: virtio-keyboard}\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - remote-viewer-spice
devices:
  input:
    - type: virtio-mouse
    - type: virtio-keyboard
    - type: usb-tablet
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let input_devices = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("devices".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("input".to_string())))
                .and_then(Value::as_sequence)
                .expect("input sequence should remain with new entries");

            assert_eq!(input_devices.len(), 1);
            assert_eq!(
                input_devices[0]
                    .as_mapping()
                    .and_then(|m| m.get(Value::String("type".to_string())))
                    .and_then(Value::as_str),
                Some("usb-tablet")
            );
        });
    }

    #[test]
    fn compacts_sparse_controller_sections_by_omitting_repeated_default_types() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("base-empty.yaml"),
                "system:\n  architecture: x86_64\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
    - base-empty
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 4096
    cpu:
        model: host
        vcpus: 4
controllers:
    scsi:
        - id: scsiA
          type: virtio-scsi-pci
        - id: scsiB
          type: virtio-scsi-pci
        - id: scsiC
          type: virtio-scsi-pci
    sata:
        - id: sataA
          type: ahci
        - id: sataB
          type: ahci
devices:
    drives:
        - id: scsi0
          path: /dev/vm1/root
          interface: scsi
          type: disk
          format: raw
          controller: scsiA
          scsi_id: 0
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let scsi = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("controllers".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("scsi".to_string())))
                .and_then(Value::as_sequence)
                .expect("scsi sequence");

            for item in scsi {
                let map = item.as_mapping().expect("controller item mapping");
                assert!(map.contains_key(Value::String("id".to_string())));
                assert!(!map.contains_key(Value::String("type".to_string())));
            }

            let roundtrip = VmConfig::from_str(&compacted).expect("compacted yaml must rehydrate");
            assert!(
                roundtrip
                    .controllers
                    .scsi
                    .iter()
                    .all(|ctrl| ctrl.r#type == "virtio-scsi-pci")
            );
            assert!(
                roundtrip
                    .controllers
                    .sata
                    .iter()
                    .all(|ctrl| ctrl.r#type == "ahci")
            );
        });
    }

    #[test]
    fn dense_multi_device_sparse_compaction_reduces_output_size_by_ten_percent() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("base-empty.yaml"),
                "system:\n  architecture: x86_64\n",
            )
            .expect("write profile");

            let input = r#"
name: dense
backend: qemu
profiles:
    - base-empty
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 8192
    cpu:
        model: host
        vcpus: 8
controllers:
    scsi:
        - id: scsi0
          type: virtio-scsi-pci
        - id: scsi1
          type: virtio-scsi-pci
        - id: scsi2
          type: virtio-scsi-pci
        - id: scsi3
          type: virtio-scsi-pci
        - id: scsi4
          type: virtio-scsi-pci
        - id: scsi5
          type: virtio-scsi-pci
    sata:
        - id: sata0
          type: ahci
        - id: sata1
          type: ahci
        - id: sata2
          type: ahci
        - id: sata3
          type: ahci
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let input_lines = input.lines().count();
            let compacted_lines = compacted.lines().count();

            assert!(
                compacted_lines * 100 <= input_lines * 90,
                "expected >=10% line reduction, got input_lines={input_lines}, compacted_lines={compacted_lines}"
            );
        });
    }
}
