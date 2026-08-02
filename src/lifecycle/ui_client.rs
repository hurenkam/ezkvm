use crate::{
    config::ezkvm::schema::DisplaySchema,
    lifecycle::host_config::HostConfig,
    runtime::{Chipset, Ivshmem, PcieBusDeviceKind, RootDeviceKind, Runtime},
};

pub fn resolve_ui_client(
    display: Option<&DisplaySchema>,
    runtime: &Runtime,
    host_config: &HostConfig,
) -> Option<(String, Vec<String>)> {
    match display {
        Some(DisplaySchema::Spice { spice }) => {
            let mut args = host_config.remote_viewer_default_args().clone();
            args.push(format!("spice://{}:{}", spice.listen(), spice.port()));
            Some((host_config.remote_viewer_path().clone(), args))
        }
        Some(DisplaySchema::Vnc { vnc }) => {
            let mut args = host_config.remote_viewer_default_args().clone();
            args.push(format!("vnc://{}:{}", vnc.listen(), vnc.port()));
            Some((host_config.remote_viewer_path().clone(), args))
        }
        Some(DisplaySchema::LookingGlass { .. }) => {
            let Some(mem_path) = ivshmem_mem_path(runtime) else {
                eprintln!("warning: Looking Glass display configured without an Ivshmem device; skipping UI client launch");
                return None;
            };

            let mut args = host_config.looking_glass_client_default_args().clone();
            args.push("-f".to_string());
            args.push(mem_path);
            Some((host_config.looking_glass_client_path().clone(), args))
        }
        Some(
            DisplaySchema::EglHeadless { .. }
            | DisplaySchema::Gtk { .. }
            | DisplaySchema::Sdl { .. },
        )
        | None => None,
    }
}

fn ivshmem_mem_path(runtime: &Runtime) -> Option<String> {
    runtime.root_devices().iter().find_map(|device| {
        if device.device_kind() != RootDeviceKind::Chipset {
            return None;
        }

        let chipset = device.as_any().downcast_ref::<Chipset>()?;
        let Chipset::Q35(q35) = chipset else {
            return None;
        };

        q35.pcie_bus().values().find_map(|pcie_device| {
            if pcie_device.device_kind() != PcieBusDeviceKind::Ivshmem {
                return None;
            }

            pcie_device
                .as_any()
                .downcast_ref::<Ivshmem>()
                .map(|ivshmem| ivshmem.mem_path().clone())
        })
    })
}
