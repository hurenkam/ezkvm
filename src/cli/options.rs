use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpec {
    pub importer: String,
    pub importer_args: Vec<String>,
    pub source_config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSpec {
    pub output_type: String,
    pub output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOptions {
    pub source: SourceSpec,
    pub output: Option<OutputSpec>,
    pub validate: bool,
    pub show_runtime: bool,
}
