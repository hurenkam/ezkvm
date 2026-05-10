use crate::config::QmpSocketType;
use crate::qemu::{QemuManager, types::QemuArgs};
use anyhow::Result;

pub(super) fn add_platform_args(manager: &QemuManager, args: &mut QemuArgs) -> Result<()> {
    let hugepages = manager.config.system.memory.hugepages.as_ref();
    let numa_nodes = &manager.config.system.cpu.numa;

    if let Some(hp) = hugepages.filter(|h| h.enabled) {
        let mem_path = hp.effective_mem_path();
        if numa_nodes.is_empty() {
            // Synthesize a single NUMA node covering all memory
            let total_mib = manager.config.system.memory.size;
            let vcpus = manager.config.system.cpu.vcpus;
            let cpus: Vec<u32> = (0..vcpus).collect();
            args.add_hugepages_memory_backend("ram-node0", total_mib, &mem_path, hp.prealloc);
            args.add_numa_node_with_memdev(0, &cpus, "ram-node0");
        } else {
            for numa in numa_nodes {
                let id = format!("ram-node{}", numa.id);
                args.add_hugepages_memory_backend(&id, numa.memory, &mem_path, hp.prealloc);
                args.add_numa_node_with_memdev(numa.id, &numa.cpus, &id);
            }
        }
    } else {
        for numa in numa_nodes {
            args.add_numa_node(numa.id, numa.memory, &numa.cpus, numa.host_node);
        }
    }

    if let Some(hyperv) = &manager.config.hyperv
        && hyperv.enabled
    {
        args.add_hyperv(
            hyperv.relaxed,
            hyperv.vapic,
            hyperv.time,
            hyperv.crash,
            hyperv.reset,
            hyperv.vendor_id.as_deref(),
            hyperv.frequencies,
            hyperv.reenlightenment,
            hyperv.tlbflush,
            hyperv.ipi,
            hyperv.spinlock_retry,
        );
    }

    Ok(())
}

pub(super) fn add_monitoring_and_identity_args(manager: &QemuManager, args: &mut QemuArgs) {
    let uses_unix_qmp = if let Some(qmp) = manager.config.options_qmp()
        && qmp.enabled
    {
        let socket_type = match qmp.socket_type {
            QmpSocketType::Tcp => "tcp",
            QmpSocketType::Unix => "unix",
        };
        args.add_qmp(qmp.socket_path.as_deref(), socket_type);
        matches!(qmp.socket_type, QmpSocketType::Unix)
    } else {
        // Auto-add a QMP unix socket so the shutdown monitor can detect
        // guest-initiated power-off and send `quit` to QEMU.
        let socket_path = manager.auto_qmp_socket_path();
        args.add_qmp(Some(&socket_path), "unix");
        true
    };

    if uses_unix_qmp {
        // Match Proxmox behavior: keep VM process alive after guest shutdown
        // so the shutdown monitor can issue an explicit `quit`.
        args.add_no_shutdown();
    }

    if let Some(smbios) = manager.config.system_smbios() {
        args.add_smbios(
            smbios.smbios_type,
            smbios.manufacturer.as_deref(),
            smbios.product.as_deref(),
            smbios.version.as_deref(),
            smbios.serial.as_deref(),
            smbios.uuid.as_deref(),
            smbios.sku.as_deref(),
            smbios.family.as_deref(),
        );

        if let Some(vm_gen_id) = &smbios.vm_generation_id {
            args.add_vm_generation_id(vm_gen_id);
        }
    }

    if let Some(applesmc) = manager.config.system_applesmc()
        && applesmc.enabled
    {
        args.add_isa_applesmc(&applesmc.osk);
    }
}
