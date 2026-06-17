use std::{fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use super::{
    Cpu, IdeAddress, IdeBus, IdeControllerApi, IdeDeviceApi, Memory, PciAddress, PciBus,
    PciControllerApi, PciDeviceApi, PcieAddress, PcieBus, PcieControllerApi, PcieDeviceApi,
    SataAddress, SataBus, SataControllerApi, SataDeviceApi, ScsiAddress, ScsiBus,
    ScsiControllerApi, ScsiDeviceApi, UsbAddress, UsbBus, UsbControllerApi, UsbDeviceApi,
};
use crate::runtime_model::{BootModel, BusRegister, Chipset, TpmApi};

pub trait ControllerApi {
    fn qemu_args(&self) -> Vec<String>;
}

#[allow(dead_code)]
#[derive(Getters, new)]
pub struct RuntimeModel {
    name: String,
    cpu: Cpu,
    memory: Memory,
    chipset: Chipset,
    boot: BootModel,
    tpm: Option<Arc<dyn TpmApi>>,
    busses: BusRegister,
}
impl RuntimeModel {
    pub fn get_pcie_bus(&self, id: PcieBus) -> Arc<dyn PcieControllerApi> {
        self.busses
            .pcie_busses()
            .get(&id)
            .expect(&format!("PCIe bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_pci_bus(&self, id: PciBus) -> Arc<dyn PciControllerApi> {
        self.busses
            .pci_busses()
            .get(&id)
            .expect(&format!("PCI bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_usb_bus(&self, id: UsbBus) -> Arc<dyn UsbControllerApi> {
        self.busses
            .usb_busses()
            .get(&id)
            .expect(&format!("USB bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_sata_bus(&self, id: SataBus) -> Arc<dyn SataControllerApi> {
        self.busses
            .sata_busses()
            .get(&id)
            .expect(&format!("SATA bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_ide_bus(&self, id: IdeBus) -> Arc<dyn IdeControllerApi> {
        self.busses
            .ide_busses()
            .get(&id)
            .expect(&format!("IDE bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_scsi_bus(&self, id: ScsiBus) -> Arc<dyn ScsiControllerApi> {
        self.busses
            .scsi_busses()
            .get(&id)
            .expect(&format!("SCSI bus with id {} does not exist", id))
            .clone()
    }
    pub fn register_pcie_device(
        &self,
        bus_id: PcieBus,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: Option<PcieAddress>,
    ) -> Result<(), String> {
        match self.busses.pcie_busses().get(&bus_id) {
            Some(controller) => {
                controller.register_pcie_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("PCIe bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_pci_device(
        &self,
        bus_id: PciBus,
        device: Arc<dyn PciDeviceApi>,
        preferred_address: Option<PciAddress>,
    ) -> Result<(), String> {
        match self.busses.pci_busses().get(&bus_id) {
            Some(controller) => {
                controller.register_pci_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("PCI bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_usb_device(
        &self,
        bus_id: UsbBus,
        device: Arc<dyn UsbDeviceApi>,
        preferred_address: Option<UsbAddress>,
    ) -> Result<(), String> {
        match self.busses.usb_busses().get(&bus_id) {
            Some(controller) => {
                controller.register_usb_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("USB bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_sata_device(
        &self,
        bus_id: SataBus,
        device: Arc<dyn SataDeviceApi>,
        preferred_address: Option<SataAddress>,
    ) -> Result<(), String> {
        match self.busses.sata_busses().get(&bus_id) {
            Some(controller) => {
                controller.register_sata_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("SATA bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_ide_device(
        &self,
        bus_id: IdeBus,
        device: Arc<dyn IdeDeviceApi>,
        preferred_address: Option<IdeAddress>,
    ) -> Result<(), String> {
        match self.busses.ide_busses().get(&bus_id) {
            Some(controller) => {
                controller.register_ide_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("IDE bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_scsi_device(
        &self,
        bus_id: ScsiBus,
        device: Arc<dyn ScsiDeviceApi>,
        preferred_address: Option<ScsiAddress>,
    ) -> Result<(), String> {
        match self.busses.scsi_busses().get(&bus_id) {
            Some(controller) => {
                controller.register_scsi_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("SCSI bus with id {} does not exist", bus_id)),
        }
    }
    pub fn start(&self) -> Result<(), String> {
        println!("qemu command: {}", self.qemu_command_display());
        Ok(())
    }

    pub fn qemu_command(&self) -> Vec<String> {
        self.generate_qemu_command()
    }

    pub fn qemu_command_display(&self) -> String {
        self.qemu_command()
            .into_iter()
            .map(|arg| shell_escape(&arg))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn generate_qemu_command(&self) -> Vec<String> {
        let mut args = vec![
            "qemu-system-x86_64".to_string(),
            "-name".to_string(),
            self.name.clone(),
        ];
        args.extend(self.cpu.qemu_args());
        args.extend(self.memory.qemu_args());
        args.extend(self.chipset.qemu_args());
        args.extend(self.boot.qemu_args());
        if let Some(tpm) = &self.tpm {
            args.extend(tpm.qemu_args(&self.name));
        }
        args.extend(self.busses.qemu_args());
        args
    }
    pub fn stop(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'stop' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
    pub fn reset(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'reset' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
    pub fn shutdown(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'shutdown' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
}

fn shell_escape(arg: &str) -> String {
    if arg.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | ',' | '=' | '+')
    }) {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

impl Display for RuntimeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RuntimeModel for VM '{}':", self.name)?;
        writeln!(f, "  {:?}", self.cpu)?;
        writeln!(f, "  {:?}", self.memory)?;
        writeln!(
            f,
            "  Chipset: {}",
            match &self.chipset {
                Chipset::Q35(_) => "Q35",
                Chipset::I440FX(_) => "I440FX",
            }
        )?;
        writeln!(f, "  {}", self.boot)?;
        writeln!(
            f,
            "  {}",
            match &self.tpm {
                Some(tpm) => format!("{}", tpm),
                None => "None".to_string(),
            }
        )?;
        writeln!(f, "  Busses: {}", self.busses)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeModel;
    use crate::runtime_config::RuntimeConfig;

    #[test]
    fn q35_supported_subset_renders_valid_command() {
        let runtime_config: RuntimeConfig = serde_json::from_value(serde_json::json!({
            "metadata": {
                "schema_version": "1.0.0",
                "vm_name": "demo"
            },
            "virtual_machine": {
                "machine": {
                    "family": "pc",
                    "chipset": "q35"
                },
                "memory": {
                    "size": 8589934592u64
                },
                "boot": {
                    "uefi": {
                        "resource": "firmware0"
                    }
                },
                "tpm": {
                    "swtpm": {
                        "version": 2.0,
                        "resource": "tpmstate0"
                    }
                },
                "devices": [
                    {
                        "pcie": {
                            "type": "pv_scsi"
                        }
                    },
                    {
                        "scsi": {
                            "type": "hdd",
                            "resource": "disk0"
                        }
                    },
                    {
                        "pcie": {
                            "type": "virtio_net",
                            "resource": "net0"
                        }
                    },
                    {
                        "ide": {
                            "type": "cdrom",
                            "resource": "iso0"
                        }
                    }
                ]
            },
            "resources": [
                {
                    "id": "firmware0",
                    "storage": {
                        "file": "/var/lib/ezkvm/efivars.fd"
                    }
                },
                {
                    "id": "tpmstate0",
                    "storage": {
                        "file": "/var/lib/ezkvm/tpmstate"
                    }
                },
                {
                    "id": "disk0",
                    "storage": {
                        "block_device": "/dev/vm/disk0"
                    }
                },
                {
                    "id": "iso0",
                    "storage": {
                        "file": "/iso/debian.iso"
                    }
                },
                {
                    "id": "net0",
                    "network": {
                        "bridge": "vmbr0"
                    }
                }
            ]
        }))
        .expect("json should parse");
        let model = RuntimeModel::try_from(runtime_config).expect("runtime model should build");
        let command = model.qemu_command();

        assert_eq!(command[0], "qemu-system-x86_64");
        assert!(command.contains(&"type=q35".to_string()));
        assert!(command.contains(&"menu=on,strict=on,reboot-timeout=1000".to_string()));
        assert!(command.contains(&"if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/var/lib/ezkvm/efivars.fd,size=540672".to_string()));
        //assert!(command.contains(&"socket,id=tpmchar,path=/var/run/ezkvm/demo.swtpm".to_string()));
        assert!(command.contains(&"pvscsi,id=scsihw0,bus=pcie.0,addr=0x0.0".to_string()));
        assert!(command.contains(&"id=drive-scsi0,file=/dev/vm/disk0,if=none,format=raw,discard=unmap,detect-zeroes=unmap".to_string()));
        assert!(
            command.contains(
                &"scsi-hd,bus=scsihw0.0,channel=0,scsi-id=0,lun=0,drive=drive-scsi0,id=scsi0"
                    .to_string()
            )
        );
        assert!(command.contains(&"bridge,id=net0f1,br=vmbr0".to_string()));
        assert!(
            command.contains(
                &"virtio-net-pci,id=net0f1,netdev=net0f1,bus=pcie.0,addr=0x0.1".to_string()
            )
        );
        assert!(
            command.contains(
                &"if=none,id=drive-ide0,file=/iso/debian.iso,format=raw,media=cdrom,readonly=on"
                    .to_string()
            )
        );
        assert!(command.contains(&"ide-cd,bus=ide.1,unit=0,drive=drive-ide0,id=ide0".to_string()));
    }

    #[test]
    fn command_display_shell_escapes_paths_with_spaces() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "demo"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 1073741824
    devices:
        - ide:
                type: cdrom
                resource: "iso0"
resources:
    - id: "iso0"
      storage:
        file: "/iso/Debian 12.iso"
"#;

        let runtime_config: RuntimeConfig = serde_yaml::from_str(yaml).expect("yaml should parse");
        let model = RuntimeModel::try_from(runtime_config).expect("runtime model should build");
        let display = model.qemu_command_display();

        assert!(display.contains(
            "'if=none,id=drive-ide0,file=/iso/Debian 12.iso,format=raw,media=cdrom,readonly=on'"
        ));
    }
}
