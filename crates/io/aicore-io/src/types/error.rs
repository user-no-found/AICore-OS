use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoProtocolErrorCode {
    InstanceNotFound,
    InstanceBindFailed,
    ClientNotAttached,
    StaleCursor,
    ActiveTurnBusy,
    ApprovalNotFound,
    ApprovalClosed,
    InvalidInput,
    Unsupported,
    NotImplementedYet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoProtocolError {
    pub code: IoProtocolErrorCode,
    pub message_zh: Option<String>,
    pub summary_en: Option<String>,
    pub retryable: bool,
}
