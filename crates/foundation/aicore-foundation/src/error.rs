use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum AicoreError {
    InvalidComponentId(String),
    InvalidInstanceId(String),
    InvalidPath(String),
    Duplicate(String),
    Missing(String),
    InvalidState(String),
    PermissionDenied(String),
}

impl Display for AicoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidComponentId(value) => write!(f, "invalid component id: {value}"),
            Self::InvalidInstanceId(value) => write!(f, "invalid instance id: {value}"),
            Self::InvalidPath(value) => write!(f, "invalid path: {value}"),
            Self::Duplicate(value) => write!(f, "duplicate: {value}"),
            Self::Missing(value) => write!(f, "missing: {value}"),
            Self::InvalidState(value) => write!(f, "invalid state: {value}"),
            Self::PermissionDenied(value) => write!(f, "permission denied: {value}"),
        }
    }
}

impl Error for AicoreError {}
