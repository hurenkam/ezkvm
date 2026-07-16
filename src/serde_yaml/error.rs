use serde::de;
use serde::ser;
use std::fmt;

/// Error type for YAML deserialization and serialization.
#[derive(Debug, Clone)]
pub enum Error {
    /// Parse error from saphyr.
    Parse(String),
    /// Custom serde error.
    Message(String),
    /// Invalid type error.
    InvalidType { expected: String, got: String },
    /// Missing required field.
    MissingField(String),
    /// Unknown field.
    UnknownField(String),
    /// Unexpected end of input.
    UnexpectedEnd,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "YAML parse error: {}", msg),
            Error::Message(msg) => write!(f, "{}", msg),
            Error::InvalidType { expected, got } => {
                write!(f, "invalid type: got {}, expected {}", got, expected)
            }
            Error::MissingField(field) => write!(f, "missing field '{}'", field),
            Error::UnknownField(field) => write!(f, "unknown field '{}'", field),
            Error::UnexpectedEnd => write!(f, "unexpected end of input"),
        }
    }
}

impl std::error::Error for Error {}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }

    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        Error::InvalidType {
            expected: exp.to_string(),
            got: unexp.to_string(),
        }
    }

    fn missing_field(field: &'static str) -> Self {
        Error::MissingField(field.to_string())
    }

    fn unknown_field(field: &str, _expected: &[&str]) -> Self {
        Error::UnknownField(field.to_string())
    }
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}
