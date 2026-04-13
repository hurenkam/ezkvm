use crate::qemu::types::QemuArgs;

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
        if let Some(last) = self.last_mut()
            && *last != "-cpu"
        {
            last.push(',');
            last.push_str(feature);
            return;
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

    /// Disable emulated VGA output.
    pub fn add_vga_none(&mut self) {
        self.push_str("-vga");
        self.push_str("none");
    }

    /// Disable QEMU's default graphical console.
    pub fn add_nographic(&mut self) {
        self.push_str("-nographic");
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
