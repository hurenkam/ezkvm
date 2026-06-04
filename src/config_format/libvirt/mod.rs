mod exporter;
mod importer;

pub struct LibvirtImporter;

pub struct LibvirtExporter;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibvirtInputArgs {
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibvirtOutputArgs {
    #[serde(rename = "output.vm")]
    pub output_vm: String,
}
