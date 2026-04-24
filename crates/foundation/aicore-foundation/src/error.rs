use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum AicoreError {
    InvalidComponentId(String),
    InvalidInstanceId(String),
}

impl Display for AicoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidComponentId(value) => write!(f, "invalid component id: {value}"),
            Self::InvalidInstanceId(value) => write!(f, "invalid instance id: {value}"),
        }
    }
}

impl Error for AicoreError {}
