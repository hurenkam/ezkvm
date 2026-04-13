use crate::qemu::types::QemuArgs;

impl QemuArgs {
    /// Add TPM device
    ///
    /// Returns an error if the backend is unsupported, rather than panicking.
    /// Supported backends: "emulator", "passthrough"
    pub fn add_tpm(
        &mut self,
        _version: &str,
        backend: &str,
        socket_path: &str,
        model: &str,
        external_swtpm: bool,
    ) -> Result<(), String> {
        add_tpm_backend_args(self, backend, socket_path, external_swtpm)?;

        // Add TPM device
        self.push_str("-device");
        self.push(format!("{},tpmdev=tpmdev", model));

        Ok(())
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
            }
            _ => socket_path
                .map(|p| format!("unix:{},server=on,wait=off", p))
                .unwrap_or_else(|| {
                    "unix:/var/run/qemu-monitor.sock,server=on,wait=off".to_string()
                }),
        };

        self.push(socket_spec);
    }

    /// Add SMBIOS system information
    #[allow(clippy::too_many_arguments)]
    pub fn add_smbios(
        &mut self,
        manufacturer: Option<&str>,
        product: Option<&str>,
        version: Option<&str>,
        serial: Option<&str>,
        uuid: Option<&str>,
        sku: Option<&str>,
        family: Option<&str>,
    ) {
        let smbios_spec =
            build_smbios_spec(manufacturer, product, version, serial, uuid, sku, family);

        self.push_str("-smbios");
        self.push(smbios_spec);
    }

    /// Add VM generation ID (for Windows Server 2016+)
    pub fn add_vm_generation_id(&mut self, generation_id: &str) {
        self.push_str("-device");
        self.push(format!("vmgenid,guid={}", generation_id));
    }

    /// Add NUMA node configuration
    pub fn add_numa_node(
        &mut self,
        node_id: u32,
        memory_mib: u32,
        cpus: &[u32],
        host_node: Option<u32>,
    ) {
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
    #[allow(clippy::too_many_arguments)]
    pub fn add_hyperv(
        &mut self,
        relaxed: bool,
        vapic: bool,
        time: bool,
        crash: bool,
        reset: bool,
        vendor_id: Option<&str>,
        frequencies: bool,
        reenlightenment: bool,
        tlbflush: bool,
        ipi: bool,
        spinlock_retry: Option<u32>,
    ) {
        self.add_hyperv_base_profile(relaxed);
        self.add_hyperv_flag_features(
            vapic,
            time,
            crash,
            reset,
            frequencies,
            reenlightenment,
            tlbflush,
            ipi,
        );
        self.add_hyperv_optional_features(vendor_id, spinlock_retry);
    }
}

fn build_smbios_spec(
    manufacturer: Option<&str>,
    product: Option<&str>,
    version: Option<&str>,
    serial: Option<&str>,
    uuid: Option<&str>,
    sku: Option<&str>,
    family: Option<&str>,
) -> String {
    let mut smbios_spec = "type=1".to_string();
    push_smbios_field(&mut smbios_spec, "manufacturer", manufacturer);
    push_smbios_field(&mut smbios_spec, "product", product);
    push_smbios_field(&mut smbios_spec, "version", version);
    push_smbios_field(&mut smbios_spec, "serial", serial);
    push_smbios_field(&mut smbios_spec, "uuid", uuid);
    push_smbios_field(&mut smbios_spec, "sku", sku);
    push_smbios_field(&mut smbios_spec, "family", family);
    smbios_spec
}

fn push_smbios_field(spec: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        spec.push_str(&format!(",{}={}", key, value));
    }
}

impl QemuArgs {
    fn add_hyperv_base_profile(&mut self, relaxed: bool) {
        if relaxed {
            self.push_str("-cpu");
            self.push_str(
                "host,+hypervisor,+invtsc,hv_relaxed,hv_spinlocks=0x1fff,hv_vapic,hv_time",
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_hyperv_flag_features(
        &mut self,
        vapic: bool,
        time: bool,
        crash: bool,
        reset: bool,
        frequencies: bool,
        reenlightenment: bool,
        tlbflush: bool,
        ipi: bool,
    ) {
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
    }

    fn add_hyperv_optional_features(
        &mut self,
        vendor_id: Option<&str>,
        spinlock_retry: Option<u32>,
    ) {
        if let Some(vendor_id) = vendor_id {
            self.add_cpu_feature(&format!("+hv_vendor_id={}", vendor_id));
        }
        if let Some(retry) = spinlock_retry {
            self.add_cpu_feature(&format!("+hv_spinlocks=0x{:x}", retry));
        }
    }
}

fn add_tpm_backend_args(
    args: &mut QemuArgs,
    backend: &str,
    socket_path: &str,
    external_swtpm: bool,
) -> Result<(), String> {
    match backend {
        "emulator" => {
            let chardev_id = "tpmchar";
            args.push_str("-chardev");
            args.push(build_tpm_chardev_spec(
                chardev_id,
                socket_path,
                external_swtpm,
            ));
            args.push_str("-tpmdev");
            args.push(format!("emulator,id=tpmdev,chardev={}", chardev_id));
            Ok(())
        }
        "passthrough" => {
            args.push_str("-tpmdev");
            args.push("passthrough,id=tpmdev".to_string());
            Ok(())
        }
        _ => Err(format!(
            "Unsupported TPM backend: '{}'. Supported backends: 'emulator', 'passthrough'",
            backend
        )),
    }
}

fn build_tpm_chardev_spec(chardev_id: &str, socket_path: &str, external_swtpm: bool) -> String {
    if external_swtpm {
        format!("socket,id={},path={}", chardev_id, socket_path)
    } else {
        format!(
            "socket,id={},server=on,wait=off,path={}",
            chardev_id, socket_path
        )
    }
}
