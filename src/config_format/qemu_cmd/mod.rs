mod exporter;
mod importer;

pub struct QemuImporter;

pub struct QemuExporter;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuInputArgs {
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuOutputArgs {
    #[serde(rename = "output.vm")]
    pub output_vm: Option<String>,
}
