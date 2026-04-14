use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    ParseError(String),
}

impl Display for ImportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::ParseError(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for ImportError {}
