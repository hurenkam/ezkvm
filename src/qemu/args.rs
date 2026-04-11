//! QEMU argument management
//!
//! Structured representation of QEMU command-line arguments.

use super::types::QemuArgs;

impl QemuArgs {
    /// Add a machine type argument
    pub fn add_machine(&mut self, machine: &str) {
        self.push_str("-machine");
        self.push(format!("type={}", machine));
    }

    /// Add VM name argument
    pub fn add_name(&mut self, name: &str) {
        self.push_str("-name");
        self.push(name.to_string());
    }

    /// Add a CPU argument
    pub fn add_cpu(&mut self, cpu: &str) {
        self.push_str("-cpu");
        self.push(cpu.to_string());
    }

    /// Add a CPU feature
    pub fn add_cpu_feature(&mut self, feature: &str) {
        // CPU features are added to the existing CPU argument
        if let Some(last) = self.last_mut() {
            if *last != "-cpu" {
                last.push(',');
                last.push_str(feature);
                return;
            }
        }
        // If no CPU argument exists, add a default one
        self.add_cpu(&format!("host{}", feature));
    }

    /// Add memory argument
    pub fn add_memory(&mut self, memory_mib: u32) {
        self.push_str("-m");
        self.push(format!("{}M", memory_mib));
    }

    /// Add SMP argument
    pub fn add_smp(&mut self, cpus: u32) {
        self.push_str("-smp");
        self.push(format!("cpus={}", cpus));
    }
    
    /// Add drive argument
    pub fn add_drive(&mut self, path: &str, interface: &str, format: &str, readonly: bool) {
        self.push_str("-drive");
        let mut spec = format!("file={},if={},format={}", path, interface, format);
        if readonly {
            spec.push_str(",readonly=on");
        }
        self.push(spec);
    }

    /// Add network argument
    pub fn add_network(&mut self, model: &str, mode: &str, mac: Option<&str>) {
        // Generate a unique ID for this network device
        let id = format!("net{}", self.len());

        self.push_str("-netdev");
        self.push(format!("id={},{}", id, mode));

        self.push_str("-device");
        let mut device_spec = format!("{},netdev={}", model, id);
        if let Some(mac_addr) = mac {
            device_spec.push_str(&format!(",mac={}", mac_addr));
        }
        self.push(device_spec);
    }

    /// Add display argument
    pub fn add_display(&mut self, display_type: &str, vram_mib: Option<u32>) {
        self.push_str("-device");
        let mut spec = display_type.to_string();
        if let Some(vram) = vram_mib {
            spec.push_str(&format!(",vram={}", vram * 1024 * 1024)); // Convert MiB to bytes
        }
        self.push(spec);
    }

    /// Add boot order argument
    pub fn add_boot_order(&mut self, order: &[String]) {
        self.push_str("-boot");
        let order_str = order.join(",");
        self.push(format!("order={}", order_str));
    }

    /// Add kernel argument
    pub fn add_kernel(&mut self, kernel: &str) {
        self.push_str("-kernel");
        self.push(kernel.to_string());
    }

    /// Add initrd argument
    pub fn add_initrd(&mut self, initrd: &str) {
        self.push_str("-initrd");
        self.push(initrd.to_string());
    }
    
    /// Add append argument
    pub fn add_append(&mut self, cmdline: &str) {
        self.push_str("-append");
        self.push(cmdline.to_string());
    }

    /// Add TPM device
    pub fn add_tpm(&mut self, _version: &str, backend: &str, socket_path: &str, model: &str, external_swtpm: bool) {
        match backend {
            "emulator" => {
                // Add chardev for TPM emulator or external swtpm socket
                let chardev_id = "tpmchar";
                self.push_str("-chardev");
                let chardev_spec = if external_swtpm {
                    format!("socket,id={},path={}", chardev_id, socket_path)
                } else {
                    format!("socket,id={},server=on,wait=off,path={}", chardev_id, socket_path)
                };
                self.push(chardev_spec);

                // Add TPM device
                self.push_str("-tpmdev");
                self.push(format!("emulator,id=tpmdev,chardev={}", chardev_id));
            }
            "passthrough" => {
                self.push_str("-tpmdev");
                self.push("passthrough,id=tpmdev".to_string());
            }
            _ => panic!("Unsupported TPM backend: {}", backend),
        }

        // Add TPM device
        self.push_str("-device");
        self.push(format!("{},tpmdev=tpmdev", model));
    }

    /// Add QEMU guest agent
    pub fn add_guest_agent(&mut self, socket_path: Option<&str>, freeze_cpu: bool, bus: Option<&str>, addr: Option<&str>) {
        // Add virtio-serial device for guest agent
        self.push_str("-device");
        let mut serial_spec = "virtio-serial-pci,id=virtio-serial0".to_string();
        if let Some(bus) = bus {
            serial_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            serial_spec.push_str(&format!(",addr={}", addr));
        }
        self.push(serial_spec);

        // Add chardev for guest agent
        let chardev_id = "qga0";
        self.push_str("-chardev");
        let chardev_spec = format!("socket,path={},server=on,wait=off,id={}",
            socket_path.unwrap_or("/var/run/qemu-server/qga.sock"), chardev_id);
        self.push(chardev_spec);

        // Add guest agent channel
        self.push_str("-device");
        let mut channel_spec = format!("virtserialport,chardev={},name=org.qemu.guest_agent.0", chardev_id);
        if freeze_cpu {
            channel_spec.push_str(",freeze=on");
        }
        self.push(channel_spec);
    }

    /// Add memory ballooning device
    pub fn add_balloon(&mut self, model: &str, free_page_reporting: bool, id: Option<&str>, bus: Option<&str>, addr: Option<&str>) {
        self.push_str("-device");
        let mut balloon_spec = model.to_string();
        if let Some(id) = id {
            balloon_spec.push_str(&format!(",id={}", id));
        }
        if let Some(bus) = bus {
            balloon_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            balloon_spec.push_str(&format!(",addr={}", addr));
        }
        if free_page_reporting {
            balloon_spec.push_str(",free-page-reporting=on");
        }
        self.push(balloon_spec);
    }

    /// Add UEFI firmware
    pub fn add_uefi(&mut self, code_path: Option<&str>, vars_path: Option<&str>, secure_boot: bool) {
        // Set firmware to UEFI
        self.push_str("-bios");
        if let Some(code) = code_path {
            // Custom OVMF firmware
            self.push(code.to_string());
        } else {
            // Use system OVMF
            self.push("/usr/share/ovmf/OVMF.fd".to_string());
        }

        // Add UEFI variables if specified
        if let Some(vars) = vars_path {
            self.push_str("-drive");
            self.push(format!("if=pflash,format=raw,file={},readonly=on", vars));
        }

        // Enable secure boot if requested
        if secure_boot {
            // Secure boot is enabled by using the correct OVMF firmware
            // The actual secure boot configuration is handled by the firmware
        }
    }

    /// Add VFIO-PCI device passthrough
    pub fn add_vfio_pci(&mut self, device: &str, id: &str, pcie: bool, x_vga: bool, romfile: Option<&str>) {
        self.push_str("-device");
        let mut vfio_spec = format!("vfio-pci,host={},id={}", device, id);
        if pcie {
            vfio_spec.push_str(",pcie=1");
        }
        if x_vga {
            vfio_spec.push_str(",x-vga=1");
        }
        if let Some(rom) = romfile {
            vfio_spec.push_str(&format!(",romfile={}", rom));
        }
        self.push(vfio_spec);
    }

    /// Add USB host device passthrough
    pub fn add_usb_host(&mut self, host_spec: &str, hostbus: Option<&str>, hostport: Option<&str>, id: &str, bus: Option<&str>, port: Option<&str>) {
        self.push_str("-device");
        let mut usb_spec = String::from("usb-host");

        if let (Some(hostbus), Some(hostport)) = (hostbus, hostport) {
            usb_spec.push_str(&format!(",hostbus={},hostport={}", hostbus, hostport));
        } else if let Some((normalized_bus, normalized_port)) = normalize_usb_host_spec(host_spec) {
            usb_spec.push_str(&format!(",hostbus={},hostport={}", normalized_bus, normalized_port));
        } else if !host_spec.trim().is_empty() {
            usb_spec.push_str(&format!(",host={}", host_spec));
        }

        usb_spec.push_str(&format!(",id={}", id));
        if let Some(bus) = bus {
            usb_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(port) = port {
            usb_spec.push_str(&format!(",port={}", port));
        }
        self.push(usb_spec);
    }

    /// Add XHCI USB controller
    pub fn add_xhci_controller(&mut self, id: &str, p2: Option<u8>, p3: Option<u8>, bus: Option<&str>, addr: Option<&str>) {
        self.push_str("-device");
        let mut controller_spec = format!("qemu-xhci,id={}", id);
        if let Some(p2) = p2 {
            controller_spec.push_str(&format!(",p2={}", p2));
        }
        if let Some(p3) = p3 {
            controller_spec.push_str(&format!(",p3={}", p3));
        }
        if let Some(bus) = bus {
            controller_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            controller_spec.push_str(&format!(",addr={}", addr));
        }
        self.push(controller_spec);
    }

    /// Add SPICE display server
    pub fn add_spice(&mut self, port: u16, addr: &str, disable_ticketing: bool, vdagent: bool, has_serial_controller: bool) {
        self.push_str("-spice");
        let mut spice_spec = format!("port={},addr={}", port, addr);
        if disable_ticketing {
            spice_spec.push_str(",disable-ticketing=on");
        }
        self.push(spice_spec);

        // Add SPICE display device
        self.push_str("-device");
        self.push("qxl-vga,id=video0".to_string());

        // Add vdagent channel for clipboard sharing
        if vdagent {
            if !has_serial_controller {
                self.push_str("-device");
                self.push("virtio-serial-pci,id=virtio-serial0".to_string());
            }
            self.push_str("-chardev");
            self.push("spicevmc,id=vdagent,name=vdagent".to_string());
            self.push_str("-device");
            self.push("virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string());
        }
    }

    /// Add a SPICE audiodev backend
    pub fn add_spice_audiodev(&mut self, id: &str) {
        self.push_str("-audiodev");
        self.push(format!("spice,id={}", id));
    }

    /// Add an audio device
    pub fn add_audio_device(&mut self, device_type: &str, id: &str, bus: Option<&str>, addr: Option<&str>, cad: Option<u8>, audiodev: Option<&str>) {
        self.push_str("-device");
        let mut device_spec = format!("{},id={}", device_type, id);

        if let Some(bus) = bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = addr {
            device_spec.push_str(&format!(",addr={}", addr));
        }

        if let Some(cad) = cad {
            device_spec.push_str(&format!(",cad={}", cad));
        }

        if let Some(audiodev) = audiodev {
            device_spec.push_str(&format!(",audiodev={}", audiodev));
        }

        self.push(device_spec);
    }

    /// Add an input device
    pub fn add_input_device(&mut self, device_type: &str) {
        self.push_str("-device");
        self.push(device_type.to_string());
    }

    /// Add Looking Glass shared memory device
    pub fn add_ivshmem(&mut self, size_mib: u32, _vectors: u32, id: &str, bus: Option<&str>, mem_path: &str) {
        self.push_str("-device");
        let mut device_spec = format!("ivshmem-plain,memdev={}", id);
        if let Some(bus) = bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }
        self.push(device_spec);

        self.push_str("-object");
        self.push(format!("memory-backend-file,id={},share=on,mem-path={},size={}M", id, mem_path, size_mib));
    }

    /// Add SCSI controller
    pub fn add_scsi_controller(&mut self, id: &str, controller_type: &str, iothread: Option<&str>, max_targets: Option<u32>, bus: Option<&str>, addr: Option<&str>) {
        self.push_str("-device");
        let mut controller_spec = format!("{},id={}", controller_type, id);
        
        if let Some(iothread) = iothread {
            controller_spec.push_str(&format!(",iothread={}", iothread));
        }
        
        if let Some(max_targets) = max_targets {
            controller_spec.push_str(&format!(",max_targets={}", max_targets));
        }

        if let Some(bus) = bus {
            controller_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = addr {
            controller_spec.push_str(&format!(",addr={}", addr));
        }
        
        self.push(controller_spec);
    }

    /// Add iSCSI disk
    pub fn add_iscsi_disk(&mut self, id: &str, portal: &str, target: &str, lun: u32, 
                         initiator: Option<&str>, username: Option<&str>, password: Option<&str>,
                         controller: Option<&str>) {
        // Add iSCSI block device
        self.push_str("-blockdev");
        let mut blockdev_spec = format!("driver=iscsi,portal={},target={},lun={},node-name={}",
            portal, target, lun, id);
        
        if let Some(initiator) = initiator {
            blockdev_spec.push_str(&format!(",initiator-name={}", initiator));
        }
        
        if let Some(username) = username {
            blockdev_spec.push_str(&format!(",user={}", username));
        }
        
        if let Some(password) = password {
            blockdev_spec.push_str(&format!(",password={}", password));
        }
        
        self.push(blockdev_spec);
        
        // Add device attachment
        self.push_str("-device");
        let device_type = if controller.is_some() { "scsi-hd" } else { "virtio-blk-pci" };
        let mut device_spec = format!("{},drive={},id={}", device_type, id, id);
        
        if let Some(controller) = controller {
            device_spec.push_str(&format!(",bus={}.0", controller));
        }
        
        self.push(device_spec);
    }

    /// Add enhanced drive with advanced options
    pub fn add_drive_enhanced(&mut self, path: &str, interface: &str, format: &str, readonly: bool,
                             discard: bool, ssd: bool, controller: Option<&str>) {
        self.push_str("-drive");
        let mut spec = format!("file={},if={},format={}", path, interface, format);
        
        if readonly {
            spec.push_str(",readonly=on");
        }
        
        if discard {
            spec.push_str(",discard=unmap");
        }
        
        if ssd {
            spec.push_str(",ssd=on");
        }
        
        if let Some(controller) = controller {
            spec.push_str(&format!(",bus={}", controller));
        }
        
        self.push(spec);
    }

    /// Add QMP monitoring
    pub fn add_qmp(&mut self, socket_path: Option<&str>, socket_type: &str) {
        self.push_str("-qmp");
        
        let socket_spec = match socket_type {
            "tcp" => {
                // QEMU should listen for QMP connections rather than attempting to connect to a pre-existing peer.
                socket_path
                    .map(|p| format!("tcp:{},server=on,wait=off", p))
                    .unwrap_or_else(|| "tcp:127.0.0.1:4444,server=on,wait=off".to_string())
            },
            _ => {
                socket_path
                    .map(|p| format!("unix:{},server=on,wait=off", p))
                    .unwrap_or_else(|| "unix:/var/run/qemu-monitor.sock,server=on,wait=off".to_string())
            }
        };
        
        self.push(socket_spec);
    }

    /// Add SMBIOS system information
    pub fn add_smbios(&mut self, manufacturer: Option<&str>, product: Option<&str>, 
                     version: Option<&str>, serial: Option<&str>, uuid: Option<&str>,
                     sku: Option<&str>, family: Option<&str>) {
        let mut smbios_spec = "type=1".to_string();
        
        if let Some(manufacturer) = manufacturer {
            smbios_spec.push_str(&format!(",manufacturer={}", manufacturer));
        }
        
        if let Some(product) = product {
            smbios_spec.push_str(&format!(",product={}", product));
        }
        
        if let Some(version) = version {
            smbios_spec.push_str(&format!(",version={}", version));
        }
        
        if let Some(serial) = serial {
            smbios_spec.push_str(&format!(",serial={}", serial));
        }
        
        if let Some(uuid) = uuid {
            smbios_spec.push_str(&format!(",uuid={}", uuid));
        }
        
        if let Some(sku) = sku {
            smbios_spec.push_str(&format!(",sku={}", sku));
        }
        
        if let Some(family) = family {
            smbios_spec.push_str(&format!(",family={}", family));
        }
        
        self.push_str("-smbios");
        self.push(smbios_spec);
    }

    /// Add VM generation ID (for Windows Server 2016+)
    pub fn add_vm_generation_id(&mut self, generation_id: &str) {
        self.push_str("-device");
        self.push(format!("vmgenid,guid={}", generation_id));
    }

    /// Add NUMA node configuration
    pub fn add_numa_node(&mut self, node_id: u32, memory_mib: u32, cpus: &[u32], host_node: Option<u32>) {
        self.push_str("-numa");
        let mut numa_spec = format!("node,nodeid={},mem={}", node_id, memory_mib);
        
        if let Some(host_node) = host_node {
            numa_spec.push_str(&format!(",memdev=mem{}", host_node));
        }
        
        self.push(numa_spec);
        
        // Add CPU assignment to this node
        for cpu in cpus {
            self.push_str("-numa");
            self.push(format!("cpu,node-id={},socket-id={}", node_id, cpu));
        }
    }

    /// Add Hyper-V enlightenments
    pub fn add_hyperv(&mut self, relaxed: bool, vapic: bool, time: bool, crash: bool,
                     reset: bool, vendor_id: Option<&str>, frequencies: bool,
                     reenlightenment: bool, tlbflush: bool, ipi: bool,
                     spinlock_retry: Option<u32>) {
        // Add Hyper-V CPU features
        if relaxed {
            self.push_str("-cpu");
            self.push_str("host,+hypervisor,+invtsc,hv_relaxed,hv_spinlocks=0x1fff,hv_vapic,hv_time");
        }
        
        // Add individual Hyper-V features
        if vapic {
            self.add_cpu_feature("+hv_vapic");
        }
        
        if time {
            self.add_cpu_feature("+hv_time");
        }
        
        if crash {
            self.add_cpu_feature("+hv_crash");
        }
        
        if reset {
            self.add_cpu_feature("+hv_reset");
        }
        
        if let Some(vendor_id) = vendor_id {
            self.add_cpu_feature(&format!("+hv_vendor_id={}", vendor_id));
        }
        
        if frequencies {
            self.add_cpu_feature("+hv_frequencies");
        }
        
        if reenlightenment {
            self.add_cpu_feature("+hv_reenlightenment");
        }
        
        if tlbflush {
            self.add_cpu_feature("+hv_tlbflush");
        }
        
        if ipi {
            self.add_cpu_feature("+hv_ipi");
        }
        
        if let Some(retry) = spinlock_retry {
            self.add_cpu_feature(&format!("+hv_spinlocks=0x{:x}", retry));
        }
    }

    /// Add enable KVM argument
    pub fn add_enable_kvm(&mut self) {
        self.push_str("-enable-kvm");
    }

    /// Add daemonize argument
    pub fn add_daemonize(&mut self) {
        self.push_str("-daemonize");
    }

    /// Add nodefaults argument
    pub fn add_nodefaults(&mut self) {
        self.push_str("-nodefaults");
    }

    /// Add a raw global option
    pub fn add_global(&mut self, spec: &str) {
        self.push_str("-global");
        self.push(spec.to_string());
    }

    /// Add RTC configuration
    pub fn add_rtc(&mut self, base: Option<&str>, driftfix: Option<&str>) {
        let mut parts = Vec::new();
        if let Some(base) = base {
            parts.push(format!("base={}", base));
        }
        if let Some(driftfix) = driftfix {
            parts.push(format!("driftfix={}", driftfix));
        }
        if !parts.is_empty() {
            self.push_str("-rtc");
            self.push(parts.join(","));
        }
    }

    /// Add pidfile argument
    pub fn add_pidfile(&mut self, path: &str) {
        self.push_str("-pidfile");
        self.push(path.to_string());
    }

    /// Add a custom argument
    pub fn add_arg(&mut self, arg: &str) {
        self.push(arg.to_string());
    }

    /// Add a key-value argument
    pub fn add_key_value(&mut self, key: &str, value: &str) {
        self.push(format!("-{}", key));
        self.push(value.to_string());
    }

    /// Build the final argument list
    pub fn build(self) -> Vec<String> {
        self.into_inner()
    }
}

fn normalize_usb_host_spec(host_spec: &str) -> Option<(&str, &str)> {
    let (hostbus, hostport) = host_spec.split_once('-')?;
    if hostbus.is_empty() || hostport.is_empty() {
        return None;
    }
    if !hostbus.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !hostport
        .split('.')
        .all(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some((hostbus, hostport))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_args() {
        let mut args = QemuArgs::new();
        args.add_memory(1024);
        args.add_smp(2);
        
        let built = args.build();
        assert_eq!(built, vec!["-m", "1024M", "-smp", "cpus=2"]);
    }
    
    #[test]
    fn test_drive_args() {
        let mut args = QemuArgs::new();
        args.add_drive("/path/to/disk.qcow2", "virtio", "qcow2", false);
        
        let built = args.build();
        assert_eq!(built, vec!["-drive", "file=/path/to/disk.qcow2,if=virtio,format=qcow2"]);
    }
    
    #[test]
    fn test_readonly_drive() {
        let mut args = QemuArgs::new();
        args.add_drive("/path/to/cd.iso", "ide", "raw", true);
        
        let built = args.build();
        assert_eq!(built, vec!["-drive", "file=/path/to/cd.iso,if=ide,format=raw,readonly=on"]);
    }

    #[test]
    fn test_tpm_uses_server_mode_for_internal_emulator() {
        let mut args = QemuArgs::new();
        args.add_tpm("2.0", "emulator", "/tmp/tpm.sock", "tpm-tis", false);

        let built = args.build();
        assert_eq!(built[0], "-chardev");
        assert_eq!(built[1], "socket,id=tpmchar,server=on,wait=off,path=/tmp/tpm.sock");
    }

    #[test]
    fn test_tpm_omits_wait_for_external_swtpm_client_mode() {
        let mut args = QemuArgs::new();
        args.add_tpm("2.0", "emulator", "/tmp/tpm.sock", "tpm-tis", true);

        let built = args.build();
        assert_eq!(built[0], "-chardev");
        assert_eq!(built[1], "socket,id=tpmchar,path=/tmp/tpm.sock");
    }

    #[test]
    fn test_qmp_uses_listening_unix_socket() {
        let mut args = QemuArgs::new();
        args.add_qmp(Some("/tmp/qmp.sock"), "unix");

        let built = args.build();
        assert_eq!(built, vec!["-qmp", "unix:/tmp/qmp.sock,server=on,wait=off"]);
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

    #[test]
    fn test_spice_audio_devices() {
        let mut args = QemuArgs::new();
        args.add_spice(5903, "0.0.0.0", true, true, false);
        args.add_spice_audiodev("spice-backend0");
        args.add_audio_device("ich9-intel-hda", "audiodev0", Some("pci.2"), Some("0xc"), None, None);
        args.add_audio_device("hda-micro", "audiodev0-codec0", Some("audiodev0.0"), None, Some(0), Some("spice-backend0"));
        args.add_audio_device("hda-duplex", "audiodev0-codec1", Some("audiodev0.0"), None, Some(1), Some("spice-backend0"));

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "spice,id=spice-backend0"));
        assert!(built.iter().any(|arg| arg == "ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc"));
        assert!(built.iter().any(|arg| arg == "hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0"));
        assert!(built.iter().any(|arg| arg == "hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0"));
        assert!(!built.iter().any(|arg| arg.contains("spice-audio")));
    }

    #[test]
    fn test_input_devices() {
        let mut args = QemuArgs::new();
        args.add_spice(5903, "0.0.0.0", true, true, false);
        args.add_input_device("virtio-mouse");
        args.add_input_device("virtio-keyboard");

        let built = args.build();
        let mouse_count = built.iter().filter(|arg| *arg == "virtio-mouse").count();
        let keyboard_count = built.iter().filter(|arg| *arg == "virtio-keyboard").count();

        assert_eq!(mouse_count, 1);
        assert_eq!(keyboard_count, 1);
        assert!(built.iter().any(|arg| arg == "virtio-serial-pci,id=virtio-serial0"));
        assert!(built.iter().any(|arg| arg == "virtserialport,chardev=vdagent,name=com.redhat.spice.0"));
    }

    #[test]
    fn test_spice_vdagent_reuses_existing_serial_controller() {
        let mut args = QemuArgs::new();
        args.add_guest_agent(Some("/var/run/qemu-server/108.qga"), false, Some("pci.0"), Some("0x8"));
        args.add_spice(5903, "0.0.0.0", true, true, true);

        let built = args.build();
        let serial_controller_count = built
            .iter()
            .filter(|arg| arg.starts_with("virtio-serial-pci"))
            .count();

        assert_eq!(serial_controller_count, 1);
        assert!(built.iter().any(|arg| arg == "virtserialport,chardev=vdagent,name=com.redhat.spice.0"));
        assert!(built.iter().any(|arg| arg == "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0"));
    }

    #[test]
    fn test_ivshmem_bus_and_mem_path() {
        let mut args = QemuArgs::new();
        args.add_ivshmem(128, 1, "ivshmem0", Some("pcie.0"), "/dev/kvmfr0");

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "ivshmem-plain,memdev=ivshmem0,bus=pcie.0"));
        assert!(built.iter().any(|arg| arg == "memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M"));
    }

    #[test]
    fn test_xhci_controller_with_placement() {
        let mut args = QemuArgs::new();
        args.add_xhci_controller("xhci", Some(15), Some(15), Some("pci.1"), Some("0x1b"));

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "qemu-xhci,id=xhci,p2=15,p3=15,bus=pci.1,addr=0x1b"));
    }

    #[test]
    fn test_usb_host_normalizes_proxmox_form() {
        let mut args = QemuArgs::new();
        args.add_usb_host("1-2.2", None, None, "usb0", Some("xhci.0"), Some("1"));

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "usb-host,hostbus=1,hostport=2.2,id=usb0,bus=xhci.0,port=1"));
    }

    #[test]
    fn test_usb_host_explicit_hostbus_hostport() {
        let mut args = QemuArgs::new();
        args.add_usb_host("", Some("1"), Some("2.2"), "usb0", Some("xhci.0"), Some("1"));

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "usb-host,hostbus=1,hostport=2.2,id=usb0,bus=xhci.0,port=1"));
    }

    #[test]
    fn test_guest_agent_with_bus_and_addr() {
        let mut args = QemuArgs::new();
        args.add_guest_agent(Some("/var/run/qemu-server/108.qga"), false, Some("pci.0"), Some("0x8"));

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "virtio-serial-pci,id=virtio-serial0,bus=pci.0,addr=0x8"));
    }

    #[test]
    fn test_balloon_with_bus_addr_and_id() {
        let mut args = QemuArgs::new();
        args.add_balloon("virtio-balloon-pci", true, Some("balloon0"), Some("pci.0"), Some("0x3"));

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "virtio-balloon-pci,id=balloon0,bus=pci.0,addr=0x3,free-page-reporting=on"));
    }

    #[test]
    fn test_scsi_controller_with_bus_and_addr() {
        let mut args = QemuArgs::new();
        args.add_scsi_controller("scsihw0", "pvscsi", None, None, Some("pci.0"), Some("0x5"));

        let built = args.build();
        assert!(built.iter().any(|arg| arg == "pvscsi,id=scsihw0,bus=pci.0,addr=0x5"));
    }
}