use super::types::QemuDevice;
use super::{default_when_missing, Config};
use applesmc::AppleSmc;
use bios::Bios;
use chipset::Chipset;
use cpu::Cpu;
use derive_getters::Getters;
use memory::Memory;
use numa::{NumaDistance, NumaNode};
use serial::Serial;
use serde::Deserialize;
use tpm::Tpm;
use virtio_rng::VirtioRng;

mod applesmc;
mod bios;
mod chipset;
mod cpu;
mod memory;
mod numa;
mod serial;
mod tpm;
mod virtio_rng;

#[allow(dead_code)]
#[derive(Deserialize, Default, Debug, Getters)]
pub struct System {
    #[serde(default, deserialize_with = "default_when_missing")]
    chipset: Box<dyn Chipset>,
    #[serde(default, deserialize_with = "default_when_missing")]
    bios: Box<dyn Bios>,
    #[serde(default, deserialize_with = "default_when_missing")]
    memory: Memory,
    #[serde(default, deserialize_with = "default_when_missing")]
    cpu: Cpu,
    #[serde(default, deserialize_with = "default_when_missing")]
    tpm: Box<dyn Tpm>,
    #[serde(default)]
    applesmc: Option<AppleSmc>,
    #[serde(default)]
    numa_nodes: Vec<NumaNode>,
    #[serde(default)]
    numa_distances: Vec<NumaDistance>,
    #[serde(default)]
    virtio_rng: Option<VirtioRng>,
    #[serde(default)]
    serial: Option<Serial>,
    #[serde(default)]
    vmgenid: Option<String>,
}

impl System {
    pub fn new(
        chipset: Box<dyn Chipset>,
        bios: Box<dyn Bios>,
        memory: Memory,
        cpu: Cpu,
        tpm: Box<dyn Tpm>,
    ) -> Self {
        Self {
            chipset,
            bios,
            memory,
            cpu,
            tpm,
            applesmc: None,
            numa_nodes: vec![],
            numa_distances: vec![],
            virtio_rng: None,
            serial: None,
            vmgenid: None,
        }
    }
}

impl QemuDevice for System {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.chipset.get_qemu_args(0));
        result.extend(self.bios.get_qemu_args(0));

        // When hugepages and NUMA nodes are both active, use per-node
        // memory-backend-file objects instead of the flat -mem-path args.
        let use_numa_backends = self.memory.has_hugepages() && !self.numa_nodes.is_empty();
        if use_numa_backends {
            let mem_path = self.memory.resolved_mem_path().unwrap_or("/dev/hugepages");
            result.push(self.memory.get_size_arg());
            for (i, node) in self.numa_nodes.iter().enumerate() {
                result.extend(node.get_qemu_args_with_memdev(i, mem_path));
            }
        } else {
            result.extend(self.memory.get_qemu_args(0));
            for node in &self.numa_nodes {
                result.extend(node.get_qemu_args(0));
            }
        }

        result.extend(self.cpu.get_qemu_args(0));
        result.extend(self.tpm.get_qemu_args(0));
        if let Some(applesmc) = &self.applesmc {
            result.extend(applesmc.get_qemu_args(0));
        }
        if let Some(rng) = &self.virtio_rng {
            result.extend(rng.get_qemu_args(0));
        }
        if let Some(serial) = &self.serial {
            result.extend(serial.get_qemu_args(0));
        }
        if let Some(ref vmgenid) = self.vmgenid {
            result.push(format!("-device vmgenid,guid={}", vmgenid));
        }
        for dist in &self.numa_distances {
            result.extend(dist.get_qemu_args(0));
        }
        result
    }

    fn pre_start(&self, config: &Config) {
        self.tpm.pre_start(config);
    }
}

#[cfg(test)]
mod tests {
    use super::bios::SeaBios;
    use super::tpm::{NoTpm, SwTpm};
    use super::*;
    use bios::Ovmf;
    use chipset::Q35;

    #[test]
    fn test_defaults() {
        let actual: System = serde_yaml::from_str(
            r#"
              "#,
        )
        .unwrap();

        let expected = System::new(
            Q35::boxed_default(),
            SeaBios::boxed_default(),
            Memory::default(),
            Cpu::default(),
            NoTpm::boxed_default(),
        );

        assert_eq!(actual.get_qemu_args(0), expected.get_qemu_args(0));
    }

    #[test]
    fn test_q35_ovmf_qemu64_swtpm() {
        let actual: System = serde_yaml::from_str(r#"
                  chipset: { type: "q35", version: "8.1" }
                  bios:    { type: "ovmf", uuid: "04d064c3-66a1-4aa7-9589-f8b3ecf91cd7", file: "/dev/vm1/vm-108-efidisk" }
                  memory:  { max: 16384, balloon: false }
                  cpu:     { model: "qemu64", sockets: 1, cores: 8, flags: "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce" }
                  tpm:     { type: "swtpm", version: 2.0, disk: "/dev/vm1/vm-108-tpmstate", socket: "/var/ezkvm/wakiza-tpm.socket" }
              "#).unwrap();

        let expected = System::new(
            Box::new(Q35::new()),
            Box::new(Ovmf::new(
                "/dev/vm1/vm-108-efidisk".to_string(),
                Some("04d064c3-66a1-4aa7-9589-f8b3ecf91cd7".to_string()),
                None,
                None,
                Ovmf::secure_boot_default(),
                Ovmf::boot_menu_default(),
                Ovmf::boot_strict_default(),
                Ovmf::reboot_timeout_default(),
                None,
                None,
            )),
            Memory::new(16384, Some(false)),
            Cpu::new(
                "qemu64".to_string(),
                1,
                8,
                "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce".to_string(),
            ),
            Box::new(SwTpm::new(
                "/dev/vm1/vm-108-tpmstate".to_string(),
                "/var/ezkvm/wakiza-tpm.socket".to_string(),
            )),
        );

        assert_eq!(actual.get_qemu_args(0), expected.get_qemu_args(0));
    }

    #[test]
    fn test_q35_ovmf_qemu64_notpm() {
        let actual: System = serde_yaml::from_str(r#"
                  chipset: { type: "q35", version: "8.1" }
                  bios:    { type: "ovmf", uuid: "c0e240a5-859a-4378-a2d9-95088f531142", file: "/dev/vm1/vm-950-disk-0" }
                  cpu:     { model: "qemu64", sockets: 1, cores: 8, flags: "+aes,enforce,+kvm_pv_eoi,+kvm_pv_unhalt,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3" }
                  memory:  { max: 16384, balloon: false }
              "#).unwrap();

        let expected = System::new(
            Box::new(Q35::new()),
            Box::new(Ovmf::new(
                "/dev/vm1/vm-950-disk-0".to_string(),
                Some("c0e240a5-859a-4378-a2d9-95088f531142".to_string()),
                None,
                None,
                Ovmf::secure_boot_default(),
                Ovmf::boot_menu_default(),
                Ovmf::boot_strict_default(),
                Ovmf::reboot_timeout_default(),
                None,
                None,
            )),
            Memory::new(16384, Some(false)),
            Cpu::new(
                "qemu64".to_string(),
                1,
                8,
                "+aes,enforce,+kvm_pv_eoi,+kvm_pv_unhalt,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3"
                    .to_string(),
            ),
            Box::<dyn Tpm>::default(),
        );

        assert_eq!(actual.get_qemu_args(0), expected.get_qemu_args(0));
    }

    #[test]
    fn test_applesmc_is_emitted_when_configured() {
        let actual: System = serde_yaml::from_str(
            r#"
                  applesmc: { osk: "my-osk-key" }
              "#,
        )
        .unwrap();

        assert!(
            actual
                .get_qemu_args(0)
                .contains(&"-device isa-applesmc,osk=my-osk-key".to_string())
        );
    }

    #[test]
    fn test_hugepages_with_numa_nodes_uses_memory_backends() {
        let system: System = serde_yaml::from_str(
            r#"
              memory:
                max: 32768
                hugepages: true
                mem_path: /run/hugepages/kvm/1048576kB
              numa_nodes:
                - nodeid: 0
                  cpus: "0-11"
                  mem: 32768
              "#,
        )
        .unwrap();

        let args = system.get_qemu_args(0);
        // Should emit -m but NOT flat -mem-path/-mem-prealloc
        assert!(args.iter().any(|a| a == "-m 32768"));
        assert!(!args.iter().any(|a| a.starts_with("-mem-path")));
        assert!(!args.iter().any(|a| a == "-mem-prealloc"));
        // Should emit the memory-backend-file object
        assert!(args.iter().any(|a| a.contains("memory-backend-file") && a.contains("id=ram-node0") && a.contains("size=32768M") && a.contains("mem-path=/run/hugepages/kvm/1048576kB")));
        // Should emit -numa node with memdev= not mem=
        assert!(args.iter().any(|a| a.contains("-numa node") && a.contains("memdev=ram-node0")));
        assert!(!args.iter().any(|a| a.contains("mem=32768M")));
    }

    #[test]
    fn test_hugepages_without_numa_nodes_uses_flat_args() {
        let system: System = serde_yaml::from_str(
            r#"
              memory:
                max: 32768
                hugepages: true
                mem_path: /run/hugepages/kvm/1048576kB
              "#,
        )
        .unwrap();

        let args = system.get_qemu_args(0);
        assert!(args.iter().any(|a| a == "-m 32768"));
        assert!(args.iter().any(|a| a.starts_with("-mem-path")));
        assert!(args.iter().any(|a| a == "-mem-prealloc"));
        assert!(!args.iter().any(|a| a.contains("memory-backend-file")));
    }

    #[test]
    fn test_vmgenid_emits_device_arg() {
        let system: System = serde_yaml::from_str(
            r#"
              vmgenid: b42d5b83-fee2-47dc-98a8-7856b18542ec
              "#,
        )
        .unwrap();

        let args = system.get_qemu_args(0);
        assert!(args.contains(&"-device vmgenid,guid=b42d5b83-fee2-47dc-98a8-7856b18542ec".to_string()));
    }

    #[test]
    fn test_vmgenid_absent_when_not_configured() {
        let system: System = serde_yaml::from_str(r#""#).unwrap();

        let args = system.get_qemu_args(0);
        assert!(!args.iter().any(|a| a.starts_with("-device vmgenid")));
    }
}
