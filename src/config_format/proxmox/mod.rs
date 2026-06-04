mod exporter;
mod importer;

pub struct ProxmoxImporter;

pub struct ProxmoxExporter;
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxmoxInputArgs {
    #[serde(rename = "input.storage")]
    pub input_storage: String,
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxmoxOutputArgs {
    #[serde(rename = "output.storage")]
    pub output_storage: String,
    #[serde(rename = "output.vm")]
    pub output_vm: String,
}
