pub mod model;
pub mod parsing;
pub mod validation;

pub use model::CanonicalDocument;
pub use parsing::{ParseError, ValidationIssue};
pub use validation::{
    ConformanceError, validate_canonical_document, validate_canonical_yaml,
};
