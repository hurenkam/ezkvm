use std::path::{Path, PathBuf};

use derive_getters::Getters;
use derive_new::new;
use serde::Deserialize;
use thiserror::Error;

use crate::config::ezkvm::schema::ResourceSchema;

#[derive(Debug, Clone, Getters, new)]
pub struct HostConfig {
    vm_dir: PathBuf,
    state_dir: PathBuf,
    qemu_path: String,
    qemu_default_args: Vec<String>,
    swtpm_path: String,
    remote_viewer_path: String,
    remote_viewer_default_args: Vec<String>,
    looking_glass_client_path: String,
    looking_glass_client_default_args: Vec<String>,
    host_resources: Vec<ResourceSchema>,
    stop_escalation_timeout_secs: Option<u64>,
}

#[derive(Debug, Error)]
pub enum HostConfigError {
    #[error("missing host config: {path}")]
    Missing { path: PathBuf },
    #[error("failed to read host config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse host config {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: crate::serde_yaml::Error,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid VM name '{name}': must not contain '/', '..', or start with '.'")]
pub struct InvalidVmNameError {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawHostConfig {
    #[serde(default)]
    vm_dir: Option<PathBuf>,
    #[serde(default)]
    state_dir: Option<PathBuf>,
    qemu_path: String,
    #[serde(default)]
    qemu_default_args: Vec<String>,
    swtpm_path: String,
    remote_viewer_path: String,
    #[serde(default)]
    remote_viewer_default_args: Vec<String>,
    looking_glass_client_path: String,
    #[serde(default)]
    looking_glass_client_default_args: Vec<String>,
    #[serde(default)]
    host_resources: Vec<ResourceSchema>,
    #[serde(default)]
    stop_escalation_timeout_secs: Option<u64>,
}

impl HostConfig {
    pub fn load(config_dir: &Path) -> Result<HostConfig, HostConfigError> {
        let path = config_dir.join("host.yaml");
        if !path.exists() {
            return Err(HostConfigError::Missing { path });
        }

        let content = std::fs::read_to_string(&path).map_err(|source| HostConfigError::Read {
            path: path.clone(),
            source,
        })?;
        let raw: RawHostConfig = crate::serde_yaml::from_str(&content).map_err(|source| {
            HostConfigError::Parse {
                path: path.clone(),
                source,
            }
        })?;

        Ok(HostConfig::new(
            raw.vm_dir.unwrap_or_else(|| config_dir.join("vm.d")),
            raw.state_dir.unwrap_or_else(|| PathBuf::from("/run/ezkvm")),
            raw.qemu_path,
            raw.qemu_default_args,
            raw.swtpm_path,
            raw.remote_viewer_path,
            raw.remote_viewer_default_args,
            raw.looking_glass_client_path,
            raw.looking_glass_client_default_args,
            raw.host_resources,
            raw.stop_escalation_timeout_secs,
        ))
    }

    pub fn resolve_vm_name<'a>(name: &'a str) -> Result<&'a str, InvalidVmNameError> {
        if name.starts_with('.') || name.starts_with('/') || name.contains('/') || name.contains("..") {
            return Err(InvalidVmNameError {
                name: name.to_string(),
            });
        }
        Ok(name)
    }

    pub fn vm_yaml_path(&self, vm_name: &str) -> Result<PathBuf, InvalidVmNameError> {
        let vm_name = Self::resolve_vm_name(vm_name)?;
        Ok(self.vm_dir.join(format!("{vm_name}.yaml")))
    }
}
