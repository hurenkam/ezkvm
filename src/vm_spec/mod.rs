pub mod model;
pub mod validation;

pub use model::CanonicalDocument;
pub use validation::{
    ConformanceError, ValidationIssue, validate_canonical_document, validate_canonical_yaml,
};
