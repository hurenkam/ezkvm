use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct LifecycleConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pidfile: Option<String>,
    #[serde(default)]
    daemonize: bool,
    #[serde(default)]
    no_shutdown: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    qmp_socket: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    qmp_event_socket: Option<String>,
}
