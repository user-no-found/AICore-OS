use super::enums::{MessageKind, SessionStatus, TurnStatus};
use aicore_foundation::{SessionId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub session_id: SessionId,
    pub title: String,
    pub created_at: Timestamp,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginTurnRequest {
    pub session_id: SessionId,
    pub turn_id: String,
    pub turn_seq: u64,
    pub started_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinishTurnRequest {
    pub turn_id: String,
    pub finished_at: Timestamp,
    pub terminal_status: TurnStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendMessageRequest {
    pub session_id: SessionId,
    pub turn_id: Option<String>,
    pub message_id: String,
    pub message_seq: u64,
    pub kind: MessageKind,
    pub content: String,
    pub created_at: Timestamp,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub title: String,
    pub status: SessionStatus,
    pub created_at: u128,
    pub updated_at: u128,
    pub turn_count: u64,
}
