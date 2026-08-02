use super::{
    host_config::HostConfig,
    qmp::QmpClient,
    stop::{LifecycleError, load_live_handle},
};

pub fn reset(host_config: &HostConfig, vm_name: &str) -> Result<(), LifecycleError> {
    let (_, handle) = load_live_handle(host_config, vm_name)?;
    let mut client = QmpClient::connect(handle.qmp_socket_path())?;
    client.system_reset_fire_and_forget()?;
    Ok(())
}
