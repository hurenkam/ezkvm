use super::Chipset;
use super::QemuDevice;
use serde::Deserialize;

const PVE_CONFIG_FILE: &str = "/usr/share/qemu-server/pve-q35-4.0.cfg";

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct PcieRootPort {
    id: String,
    chassis: u8,
    slot: u8,
    addr: String,
    bus: String,
}

impl Default for PcieRootPort {
    fn default() -> Self {
        Self {
            id: "pcie.1".to_string(),
            chassis: 1,
            slot: 1,
            addr: "0x2".to_string(),
            bus: "pcie.0".to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Q35 {
    machine: String,
    pcie_root_ports: Vec<PcieRootPort>,
    xhci_enabled: bool,
    xhci_bus: String,
    xhci_addr: String,
}

impl Q35 {
    fn machine_default() -> String {
        "q35".to_string()
    }

    fn xhci_enabled_default() -> bool {
        true
    }

    fn xhci_bus_default() -> String {
        "pci.1".to_string()
    }

    fn xhci_addr_default() -> String {
        "0x1b".to_string()
    }

    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self::default())
    }
}

impl Default for Q35 {
    fn default() -> Self {
        Self {
            machine: Self::machine_default(),
            pcie_root_ports: vec![],
            xhci_enabled: Self::xhci_enabled_default(),
            xhci_bus: Self::xhci_bus_default(),
            xhci_addr: Self::xhci_addr_default(),
        }
    }
}

impl QemuDevice for Q35 {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![
            format!("-machine {}", self.machine),
            "-rtc driftfix=slew,base=localtime".to_string(),
            "-global kvm-pit.lost_tick_policy=discard".to_string(),
            format!("-readconfig {}", PVE_CONFIG_FILE),
        ];

        for rp in &self.pcie_root_ports {
            result.push(format!(
                "-device pcie-root-port,id={},chassis={},slot={},bus={},addr={}",
                rp.id, rp.chassis, rp.slot, rp.bus, rp.addr
            ));
        }

        if self.xhci_enabled {
            result.push(format!(
                "-device qemu-xhci,p2=15,p3=15,id=xhci,bus={},addr={}",
                self.xhci_bus, self.xhci_addr
            ));
        }

        result.push("-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b".to_string());

        result
    }
}

#[typetag::deserialize(name = "q35")]
impl Chipset for Q35 {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let q35 = Q35::default();
        assert_eq!(
            q35.get_qemu_args(0),
            vec![
                "-machine q35".to_string(),
                "-rtc driftfix=slew,base=localtime".to_string(),
                "-global kvm-pit.lost_tick_policy=discard".to_string(),
                "-readconfig /usr/share/qemu-server/pve-q35-4.0.cfg".to_string(),
                "-device qemu-xhci,p2=15,p3=15,id=xhci,bus=pci.1,addr=0x1b".to_string(),
                "-iscsi initiator-name=iqn.1993-08.org.debian:01:39407ad058b".to_string(),
            ]
        );
    }

    #[test]
    fn test_root_ports_and_xhci_customization() {
        let input = r#"
            pcie_root_ports:
              - id: rp0
                chassis: 10
                slot: 1
                bus: pcie.0
                addr: 0x2
              - id: rp1
                chassis: 11
                slot: 2
                bus: pcie.0
                addr: 0x3
            xhci_enabled: false
        "#;

        let q35: Q35 = serde_yaml::from_str(input).unwrap();
        let args = q35.get_qemu_args(0);
        assert!(
            args.contains(&"-device pcie-root-port,id=rp0,chassis=10,slot=1,bus=pcie.0,addr=0x2".to_string())
        );
        assert!(
            args.contains(&"-device pcie-root-port,id=rp1,chassis=11,slot=2,bus=pcie.0,addr=0x3".to_string())
        );
        assert!(!args.iter().any(|a| a.contains("qemu-xhci")));
    }
}
