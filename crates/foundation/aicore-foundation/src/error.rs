use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum AicoreError {
    InvalidComponentId(String),
    InvalidInstanceId(String),
    InvalidId { kind: String, value: String },
    InvalidPath(String),
    Duplicate(String),
    Missing(String),
    InvalidState(String),
    PermissionDenied(String),
    QueueFull(String),
    Cancelled(String),
    Timeout(String),
    VersionMismatch(String),
    Unavailable(String),
    Conflict(String),
}

pub type AicoreResult<T> = Result<T, AicoreError>;

impl Display for AicoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidComponentId(value) => write!(f, "invalid component id: {value}"),
            Self::InvalidInstanceId(value) => write!(f, "invalid instance id: {value}"),
            Self::InvalidId { kind, value } => write!(f, "invalid {kind}: {value}"),
            Self::InvalidPath(value) => write!(f, "invalid path: {value}"),
            Self::Duplicate(value) => write!(f, "duplicate: {value}"),
            Self::Missing(value) => write!(f, "missing: {value}"),
            Self::InvalidState(value) => write!(f, "invalid state: {value}"),
            Self::PermissionDenied(value) => write!(f, "permission denied: {value}"),
            Self::QueueFull(value) => write!(f, "queue full: {value}"),
            Self::Cancelled(value) => write!(f, "cancelled: {value}"),
            Self::Timeout(value) => write!(f, "timeout: {value}"),
            Self::VersionMismatch(value) => write!(f, "version mismatch: {value}"),
            Self::Unavailable(value) => write!(f, "unavailable: {value}"),
            Self::Conflict(value) => write!(f, "conflict: {value}"),
        }
    }
}

impl Error for AicoreError {}
