use serde::{Deserialize, Serialize};

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_true(v: &bool) -> bool {
    *v
}

/// IOMMU / vIOMMU device configuration.
///
/// When present, the QEMU command builder emits a `-device intel-iommu` (or
/// AMD equivalent) with the requested options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IommuConfig {
    /// IOMMU model: "intel" or "amd" (default: "intel")
    #[serde(default = "default_iommu_type")]
    pub r#type: String,

    /// Enable interrupt remapping (intremap)
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub intremap: bool,

    /// Enable caching mode (required for passthrough when intremap is on)
    #[serde(default, skip_serializing_if = "is_false")]
    pub caching_mode: bool,

    /// Enable Extended Context (EIM) support
    #[serde(default, skip_serializing_if = "is_false")]
    pub eim: bool,

    /// Device tree name / id
    #[serde(default = "default_iommu_id", skip_serializing_if = "is_default_id")]
    pub id: String,
}

fn default_iommu_type() -> String {
    "intel".to_string()
}

fn default_iommu_id() -> String {
    "iommu0".to_string()
}

fn is_default_id(id: &str) -> bool {
    id == "iommu0"
}

fn default_true() -> bool {
    true
}

impl Default for IommuConfig {
    fn default() -> Self {
        Self {
            r#type: default_iommu_type(),
            intremap: true,
            caching_mode: false,
            eim: false,
            id: default_iommu_id(),
        }
    }
}
