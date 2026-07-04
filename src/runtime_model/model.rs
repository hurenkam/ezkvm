#![allow(clippy::too_many_arguments)]
use std::{fmt::Display, sync::Arc};

use derive_getters::Getters;

use super::{
    Cpu, IdeAddress, IdeBus, IdeControllerApi, IdeDeviceApi, Memory, PciAddress, PciBus,
    PciControllerApi, PciDeviceApi, PcieAddress, PcieBus, PcieControllerApi, PcieDeviceApi,
    SataAddress, SataBus, SataControllerApi, SataDeviceApi, ScsiAddress, ScsiBus,
    ScsiControllerApi, ScsiDeviceApi, UsbAddress, UsbBus, UsbControllerApi, UsbDeviceApi,
};
use crate::runtime_model::{
    AudioApi, BootModel, BusRegister, Chipset, DisplayApi, GuestAgentApi, LifecycleConfig, TpmApi,
};

use super::qmp::execute_qmp_command;

#[allow(dead_code)]
#[derive(Getters)]
pub struct RuntimeModel {
    name: String,
    cpu: Cpu,
    memory: Memory,
    chipset: Chipset,
    boot: BootModel,
    smbios_uuid: Option<String>,
    vmgenid: Option<String>,
    tpm: Option<Arc<dyn TpmApi>>,
    display: Option<Arc<dyn DisplayApi>>,
    audio: Option<Arc<dyn AudioApi>>,
    guest_agent: Option<Arc<dyn GuestAgentApi>>,
    lifecycle_config: Option<LifecycleConfig>,
    busses: BusRegister,
}
#[allow(dead_code)] // TODO: wire to CLI
impl RuntimeModel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        cpu: Cpu,
        memory: Memory,
        chipset: Chipset,
        boot: BootModel,
        smbios_uuid: Option<String>,
        vmgenid: Option<String>,
        tpm: Option<Arc<dyn TpmApi>>,
        display: Option<Arc<dyn DisplayApi>>,
        audio: Option<Arc<dyn AudioApi>>,
        guest_agent: Option<Arc<dyn GuestAgentApi>>,
        busses: BusRegister,
    ) -> Self {
        Self {
            name,
            cpu,
            memory,
            chipset,
            boot,
            smbios_uuid,
            vmgenid,
            tpm,
            display,
            audio,
            guest_agent,
            lifecycle_config: None,
            busses,
        }
    }

    pub fn with_lifecycle_config(mut self, lifecycle_config: Option<LifecycleConfig>) -> Self {
        self.lifecycle_config = lifecycle_config;
        self
    }

    pub fn get_pcie_bus(&self, id: PcieBus) -> Arc<dyn PcieControllerApi> {
        self.busses
            .pcie_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("PCIe bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_pci_bus(&self, id: PciBus) -> Arc<dyn PciControllerApi> {
        self.busses
            .pci_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("PCI bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_usb_bus(&self, id: UsbBus) -> Arc<dyn UsbControllerApi> {
        self.busses
            .usb_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("USB bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_sata_bus(&self, id: SataBus) -> Arc<dyn SataControllerApi> {
        self.busses
            .sata_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("SATA bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_ide_bus(&self, id: IdeBus) -> Arc<dyn IdeControllerApi> {
        self.busses
            .ide_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("IDE bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_scsi_bus(&self, id: ScsiBus) -> Arc<dyn ScsiControllerApi> {
        self.busses
            .scsi_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("SCSI bus with id {} does not exist", id))
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
        println!(
            "lifecycle action 'start' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
    pub fn stop(&self) -> Result<(), String> {
        let qmp_socket = self.qmp_socket()?;
        execute_qmp_command(qmp_socket, "quit", true)
    }
    pub fn reset(&self) -> Result<(), String> {
        let qmp_socket = self.qmp_socket()?;
        execute_qmp_command(qmp_socket, "system_reset", false)
    }
    pub fn shutdown(&self) -> Result<(), String> {
        let qmp_socket = self.qmp_socket()?;
        execute_qmp_command(qmp_socket, "system_powerdown", false)
    }

    fn qmp_socket(&self) -> Result<&str, String> {
        let Some(config) = self.lifecycle_config().as_ref() else {
            return Err(format!(
                "lifecycle action requires lifecycle configuration for vm '{}'",
                self.name
            ));
        };

        config.qmp_socket().as_deref().ok_or_else(|| {
            format!(
                "lifecycle action requires 'qmp_socket' in lifecycle configuration for vm '{}'",
                self.name
            )
        })
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
        writeln!(
            f,
            "  Display: {}",
            match &self.display {
                Some(d) => format!("{}", d),
                None => "none".to_string(),
            }
        )?;
        writeln!(
            f,
            "  Audio: {}",
            match &self.audio {
                Some(a) => format!("{}", a),
                None => "none".to_string(),
            }
        )?;
        writeln!(
            f,
            "  Guest Agent: {}",
            match &self.guest_agent {
                Some(ga) => format!("{}", ga),
                None => "none".to_string(),
            }
        )?;
        writeln!(
            f,
            "  Lifecycle: {}",
            match &self.lifecycle_config {
                Some(_) => "configured".to_string(),
                None => "none".to_string(),
            }
        )?;
        writeln!(f, "  Busses: {}", self.busses)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixListener,
        path::PathBuf,
        sync::{Arc, Mutex},
        thread,
    };

    use serde_json::Value;

    use crate::runtime_model::{
        BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, LifecycleConfig, Memory,
        Q35Chipset, SeaBiosModel,
    };

    use super::RuntimeModel;

    #[derive(Clone, Copy)]
    enum QmpServerBehavior {
        ReplyToCommand,
        DisconnectAfterCommand,
    }

    fn test_runtime(socket_path: Option<String>) -> RuntimeModel {
        let mut bus_register = BusRegister::new();
        RuntimeModel::new(
            "qmp-test-vm".to_string(),
            Cpu::new(CpuModel::Host, 2, 1, 1),
            Memory::megabytes(2048),
            Chipset::Q35(Q35Chipset::new(&mut bus_register)),
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            None,
            None,
            None,
            None,
            None,
            None,
            bus_register,
        )
        .with_lifecycle_config(Some(LifecycleConfig::new(
            None,
            false,
            false,
            socket_path,
            None,
        )))
    }

    fn spawn_mock_qmp_server(
        socket_path: &std::path::Path,
        behavior: QmpServerBehavior,
    ) -> (thread::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let recorded_clone = Arc::clone(&recorded);
        let socket_path = socket_path.to_path_buf();

        let listener = UnixListener::bind(&socket_path)
            .unwrap_or_else(|e| panic!("failed to bind mock QMP socket: {e}"));

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener
                .accept()
                .unwrap_or_else(|e| panic!("failed to accept QMP client: {e}"));
            let mut reader = BufReader::new(
                stream
                    .try_clone()
                    .unwrap_or_else(|e| panic!("failed to clone stream: {e}")),
            );

            stream
                .write_all(
                    b"{\"QMP\":{\"version\":{\"qemu\":{\"major\":8,\"minor\":2,\"micro\":0},\"package\":\"\"},\"capabilities\":[]}}\n",
                )
                .unwrap_or_else(|e| panic!("failed to write greeting: {e}"));

            let capabilities = read_execute_command(&mut reader)
                .unwrap_or_else(|e| panic!("failed reading capabilities command: {e}"));
            assert_eq!(capabilities, "qmp_capabilities");
            stream
                .write_all(b"{\"return\":{}}\n")
                .unwrap_or_else(|e| panic!("failed to reply capabilities: {e}"));

            let command = read_execute_command(&mut reader)
                .unwrap_or_else(|e| panic!("failed reading runtime command: {e}"));
            recorded_clone
                .lock()
                .unwrap_or_else(|e| panic!("failed to lock recordings: {e}"))
                .push(command);

            match behavior {
                QmpServerBehavior::ReplyToCommand => {
                    stream
                        .write_all(b"{\"return\":{}}\n")
                        .unwrap_or_else(|e| panic!("failed to reply command: {e}"));
                }
                QmpServerBehavior::DisconnectAfterCommand => {}
            }
        });

        (handle, recorded)
    }

    fn read_execute_command(
        reader: &mut BufReader<std::os::unix::net::UnixStream>,
    ) -> Result<String, String> {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("failed to read line: {e}"))?;
        if line.trim().is_empty() {
            return Err("received empty command line".to_string());
        }
        let value: Value =
            serde_json::from_str(line.trim()).map_err(|e| format!("invalid json: {e}"))?;
        value
            .get("execute")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| "missing execute command".to_string())
    }

    fn socket_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "ezkvm-{name}-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        path
    }

    #[test]
    fn shutdown_sends_system_powerdown_to_qmp() {
        let path = socket_path("shutdown");
        let (handle, recorded) = spawn_mock_qmp_server(&path, QmpServerBehavior::ReplyToCommand);

        let runtime = test_runtime(Some(path.to_string_lossy().to_string()));
        runtime
            .shutdown()
            .expect("shutdown should be sent over QMP");

        handle.join().expect("mock server thread should complete");
        let commands = recorded
            .lock()
            .unwrap_or_else(|e| panic!("failed to lock recordings: {e}"));
        assert_eq!(commands.as_slice(), ["system_powerdown"]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reset_sends_system_reset_to_qmp() {
        let path = socket_path("reset");
        let (handle, recorded) = spawn_mock_qmp_server(&path, QmpServerBehavior::ReplyToCommand);

        let runtime = test_runtime(Some(path.to_string_lossy().to_string()));
        runtime.reset().expect("reset should be sent over QMP");

        handle.join().expect("mock server thread should complete");
        let commands = recorded
            .lock()
            .unwrap_or_else(|e| panic!("failed to lock recordings: {e}"));
        assert_eq!(commands.as_slice(), ["system_reset"]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stop_sends_quit_and_tolerates_disconnect() {
        let path = socket_path("stop");
        let (handle, recorded) =
            spawn_mock_qmp_server(&path, QmpServerBehavior::DisconnectAfterCommand);

        let runtime = test_runtime(Some(path.to_string_lossy().to_string()));
        runtime
            .stop()
            .expect("stop should tolerate QMP disconnect after quit");

        handle.join().expect("mock server thread should complete");
        let commands = recorded
            .lock()
            .unwrap_or_else(|e| panic!("failed to lock recordings: {e}"));
        assert_eq!(commands.as_slice(), ["quit"]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn lifecycle_action_requires_qmp_socket() {
        let runtime = test_runtime(None);
        let error = runtime
            .shutdown()
            .expect_err("shutdown should fail without qmp socket");
        assert!(error.contains("qmp_socket"));
    }
}
