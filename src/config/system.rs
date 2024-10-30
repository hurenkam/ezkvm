use crate::config::qemu_device::QemuDevice;
use crate::config::system::bios::Bios;
use crate::config::system::chipset::Chipset;
use crate::config::system::cpu::Cpu;
use crate::config::system::memory::Memory;
use crate::config::system::tpm::Tpm;
use crate::config::{default_when_missing, Config};
use derive_getters::Getters;
use serde::Deserialize;
use typetag::serde;

mod bios;
mod chipset;
mod cpu;
mod memory;
mod tpm;

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
        }
    }
}

impl QemuDevice for System {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];
        result.extend(self.chipset.get_qemu_args(0));
        result.extend(self.bios.get_qemu_args(0));
        result.extend(self.memory.get_qemu_args(0));
        result.extend(self.cpu.get_qemu_args(0));
        result.extend(self.tpm.get_qemu_args(0));
        result
    }

    fn pre_start(&self, config: &Config) {
        self.tpm.pre_start(config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::system::bios::SeaBios;
    use crate::config::system::tpm::{NoTpm, SwTpm};
    use bios::OVMF;
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
            Box::new(OVMF::new(
                "/dev/vm1/vm-108-efidisk".to_string(),
                "04d064c3-66a1-4aa7-9589-f8b3ecf91cd7".to_string(),
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
            Box::new(OVMF::new(
                "/dev/vm1/vm-950-disk-0".to_string(),
                "c0e240a5-859a-4378-a2d9-95088f531142".to_string(),
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
}
