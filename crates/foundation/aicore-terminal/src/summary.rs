use serde::Serialize;

use crate::symbols::Status;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepSummary {
    pub name: String,
    pub status: Status,
    pub warning_count: usize,
}

impl StepSummary {
    pub fn new(name: &str, status: Status, warning_count: usize) -> Self {
        Self {
            name: name.to_string(),
            status,
            warning_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunSummary {
    pub name: String,
    pub status: Status,
    pub step_count: usize,
    pub warning_count: usize,
}

impl RunSummary {
    pub fn new(name: &str, status: Status, step_count: usize, warning_count: usize) -> Self {
        Self {
            name: name.to_string(),
            status,
            step_count,
            warning_count,
        }
    }
}
