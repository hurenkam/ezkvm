#[derive(Debug, Clone, PartialEq, Default)]
pub struct EzkvmImportResult {
    pub yaml: String,
    pub mapped_keys: Vec<String>,
    pub skipped_keys: Vec<String>,
    pub warnings: Vec<String>,
}
