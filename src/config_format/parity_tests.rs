//! Cross-format parity tests for shared ezkvm <-> Proxmox supported subsets.

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use crate::{
        config_format::{
            RuntimeBuilder, SchemaBuilder,
            proxmox::{
                ProxmoxConfigSchema, ProxmoxRuntimeBuilder, ProxmoxSchemaBuilder,
                ProxmoxStorageConfig,
            },
        },
        runtime_model::{Chipset, RuntimeModel},
    };

    #[derive(Debug, PartialEq, Eq)]
    struct SupportedParitySnapshot {
        name: String,
        chipset: &'static str,
        cpu_model: String,
        cores: u8,
        sockets: u8,
        memory_mb: u64,
        uses_uefi: bool,
        scsi_disk_count: usize,
        virtio_net_count: usize,
        guest_agent_enabled: bool,
    }

    fn workspace_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn parse_storage_cfg(storage_cfg_path: &PathBuf) -> ProxmoxStorageConfig {
        let storage_text = fs::read_to_string(storage_cfg_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", storage_cfg_path.display()));
        ProxmoxStorageConfig::parse(&storage_text)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", storage_cfg_path.display()))
    }

    fn parse_conf_schema(conf_path: &PathBuf) -> ProxmoxConfigSchema {
        let conf_text = fs::read_to_string(conf_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", conf_path.display()));
        ProxmoxConfigSchema::parse(&conf_text)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", conf_path.display()))
    }

    fn runtime_from_conf(conf_path: &PathBuf, storage_cfg_path: &PathBuf) -> RuntimeModel {
        let schema = parse_conf_schema(conf_path);
        let storage_cfg = parse_storage_cfg(storage_cfg_path);
        ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_cfg)
            .with_schema(schema)
            .build()
            .unwrap_or_else(|e| panic!("failed to build runtime from {}: {e}", conf_path.display()))
    }

    fn cpu_topology(cpu_args: &[String]) -> (String, u8, u8) {
        let mut model = "host".to_string();
        let mut cores: u8 = 1;
        let mut sockets: u8 = 1;

        let mut iter = cpu_args.iter();
        while let Some(arg) = iter.next() {
            if arg == "-cpu"
                && let Some(next) = iter.next()
            {
                model = next.clone();
            }
            if arg == "-smp"
                && let Some(next) = iter.next()
            {
                for token in next.split(',') {
                    if let Some(value) = token.strip_prefix("cores=") {
                        cores = value.parse::<u8>().unwrap_or(1);
                    }
                    if let Some(value) = token.strip_prefix("sockets=") {
                        sockets = value.parse::<u8>().unwrap_or(1);
                    }
                }
            }
        }

        (model, cores, sockets)
    }

    fn memory_mb(runtime: &RuntimeModel) -> u64 {
        for pair in runtime.memory().qemu_args(runtime.cpu()).windows(2) {
            if let [flag, value] = pair
                && flag == "-m"
            {
                return value
                    .trim_end_matches('M')
                    .parse::<u64>()
                    .unwrap_or_else(|e| panic!("failed to parse -m value '{value}': {e}"));
            }
        }
        panic!("runtime memory args did not contain -m");
    }

    fn qemu_device_count(command: &[String], needle: &str) -> usize {
        command.iter().filter(|arg| arg.contains(needle)).count()
    }

    fn snapshot_supported_subset(runtime: &RuntimeModel) -> SupportedParitySnapshot {
        let (cpu_model, cores, sockets) = cpu_topology(&runtime.cpu().qemu_args());
        let command = runtime.qemu_command();

        SupportedParitySnapshot {
            name: runtime.name().to_string(),
            chipset: match runtime.chipset() {
                Chipset::Q35(_) => "q35",
                Chipset::I440FX(_) => "i440fx",
            },
            cpu_model,
            cores,
            sockets,
            memory_mb: memory_mb(runtime),
            uses_uefi: command.iter().any(|arg| arg.contains("if=pflash,unit=1")),
            scsi_disk_count: qemu_device_count(&command, "scsi-hd"),
            virtio_net_count: qemu_device_count(&command, "virtio-net-pci"),
            guest_agent_enabled: command
                .iter()
                .any(|arg| arg.contains("org.qemu.guest_agent.0")),
        }
    }

    fn assert_basic_startup_shape(command: &[String], context: &str) {
        let has_name = command
            .windows(2)
            .any(|w| matches!(w, [flag, _] if flag == "-name"));
        let has_cpu = command
            .windows(2)
            .any(|w| matches!(w, [flag, _] if flag == "-cpu"));
        let has_smp = command
            .windows(2)
            .any(|w| matches!(w, [flag, _] if flag == "-smp"));
        let has_memory = command
            .windows(2)
            .any(|w| matches!(w, [flag, _] if flag == "-m"));

        assert_eq!(
            command.first().map(String::as_str),
            Some("qemu-system-x86_64"),
            "{context}: missing qemu executable"
        );
        assert!(has_name, "{context}: missing -name argument");
        assert!(has_cpu, "{context}: missing -cpu argument");
        assert!(has_smp, "{context}: missing -smp argument");
        assert!(has_memory, "{context}: missing -m argument");
    }

    fn sorted_conf_files(host_dir: &PathBuf) -> Vec<PathBuf> {
        let mut confs: Vec<PathBuf> = fs::read_dir(host_dir)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", host_dir.display()))
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let is_conf = path.extension().is_some_and(|ext| ext == "conf");
                if is_conf { Some(path) } else { None }
            })
            .collect();
        confs.sort();
        confs
    }

    fn file_backing_paths_from_qemu_command(command: &[String]) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        let mut iter = command.iter().peekable();
        while let Some(arg) = iter.next() {
            if arg == "-readconfig"
                && let Some(path) = iter.next()
            {
                paths.push(PathBuf::from(path));
            }

            if let Some(file_path) = arg
                .split(',')
                .find_map(|segment| segment.strip_prefix("file="))
            {
                paths.push(PathBuf::from(file_path));
            }
        }

        paths
    }

    #[test]
    fn proxmox_round_trip_preserves_supported_subset_for_reference_fixtures() {
        let fixtures = [
            "input/zbp-server-mh2/301.conf",
            "input/zbp-server-mh2/103.conf",
            "input/felucia/108.conf",
            "input/coruscant/101.conf",
        ];

        for fixture in fixtures {
            let conf_path = workspace_path(fixture);
            let storage_cfg_path = conf_path
                .parent()
                .expect("fixture should have a parent directory")
                .join("storage.cfg");

            let original_runtime = runtime_from_conf(&conf_path, &storage_cfg_path);
            let original_snapshot = snapshot_supported_subset(&original_runtime);

            let storage_cfg = parse_storage_cfg(&storage_cfg_path);
            let round_trip_schema = ProxmoxSchemaBuilder::default()
                .with_storage_config(storage_cfg.clone())
                .with_runtime(original_runtime)
                .build()
                .unwrap_or_else(|e| {
                    panic!("failed to build schema from {}: {e}", conf_path.display())
                });

            let round_trip_runtime = ProxmoxRuntimeBuilder::default()
                .with_storage_config(storage_cfg)
                .with_schema(round_trip_schema)
                .build()
                .unwrap_or_else(|e| {
                    panic!(
                        "failed to rebuild runtime after round-trip for {}: {e}",
                        conf_path.display()
                    )
                });

            let round_trip_snapshot = snapshot_supported_subset(&round_trip_runtime);
            assert_eq!(
                original_snapshot,
                round_trip_snapshot,
                "supported subset changed after round-trip for {}",
                conf_path.display()
            );
        }
    }

    #[test]
    fn proxmox_input_corpus_builds_runtime_and_basic_startup_shape() {
        let input_root = workspace_path("input");
        let mut checked = 0usize;

        let mut host_dirs: Vec<PathBuf> = fs::read_dir(&input_root)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", input_root.display()))
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_dir() { Some(path) } else { None }
            })
            .collect();
        host_dirs.sort();

        for host_dir in host_dirs {
            let conf_files = sorted_conf_files(&host_dir);
            if conf_files.is_empty() {
                continue;
            }

            let storage_cfg_path = host_dir.join("storage.cfg");
            assert!(
                storage_cfg_path.exists(),
                "host directory {} has .conf files but no storage.cfg",
                host_dir.display()
            );

            for conf_path in conf_files {
                let runtime = runtime_from_conf(&conf_path, &storage_cfg_path);
                let command = runtime.qemu_command();
                assert_basic_startup_shape(&command, &conf_path.display().to_string());
                checked += 1;
            }
        }

        assert!(
            checked > 0,
            "expected at least one proxmox fixture to be checked"
        );
    }

    #[test]
    #[ignore = "host-gated; enable with EZKVM_ENABLE_HOST_START_SMOKE=1"]
    fn proxmox_host_gated_preflight_and_start_smoke_for_input_corpus() {
        if env::var("EZKVM_ENABLE_HOST_START_SMOKE").ok().as_deref() != Some("1") {
            eprintln!("set EZKVM_ENABLE_HOST_START_SMOKE=1 to run host-gated startup smoke checks");
            return;
        }

        let input_root = workspace_path("input");
        let mut host_dirs: Vec<PathBuf> = fs::read_dir(&input_root)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", input_root.display()))
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_dir() { Some(path) } else { None }
            })
            .collect();
        host_dirs.sort();

        let mut checked = 0usize;
        let mut started = 0usize;
        let mut skipped = 0usize;

        for host_dir in host_dirs {
            let conf_files = sorted_conf_files(&host_dir);
            if conf_files.is_empty() {
                continue;
            }

            let storage_cfg_path = host_dir.join("storage.cfg");
            if !storage_cfg_path.exists() {
                eprintln!("skipping host {} (missing storage.cfg)", host_dir.display());
                continue;
            }

            for conf_path in conf_files {
                checked += 1;
                let runtime = runtime_from_conf(&conf_path, &storage_cfg_path);
                let command = runtime.qemu_command();
                let missing_paths: Vec<PathBuf> = file_backing_paths_from_qemu_command(&command)
                    .into_iter()
                    .filter(|path| !path.exists())
                    .collect();

                if !missing_paths.is_empty() {
                    eprintln!(
                        "skipping {} (missing host paths: {})",
                        conf_path.display(),
                        missing_paths
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    skipped += 1;
                    continue;
                }

                runtime.start().unwrap_or_else(|e| {
                    panic!("failed startup smoke for {}: {e}", conf_path.display())
                });
                started += 1;
            }
        }

        assert!(
            checked > 0,
            "expected at least one proxmox config in input corpus"
        );
        assert!(
            started > 0,
            "no fixture had sufficient host resources to run startup smoke"
        );
        eprintln!(
            "host-gated startup smoke summary: checked={}, started={}, skipped={}",
            checked, started, skipped
        );
    }
}
