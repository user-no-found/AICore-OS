use super::enums::{
    ControlEventKind, LedgerWriteKind, MessageKind, RuntimeStatus, SessionStatus, TurnStatus,
};
use aicore_foundation::{InstanceId, SessionId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub instance_id: InstanceId,
    pub session_id: SessionId,
    pub title: String,
    pub created_at: Timestamp,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginTurnRequest {
    pub instance_id: InstanceId,
    pub session_id: SessionId,
    pub turn_id: String,
    pub turn_seq: u64,
    pub started_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinishTurnRequest {
    pub instance_id: InstanceId,
    pub turn_id: String,
    pub finished_at: Timestamp,
    pub terminal_status: TurnStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendMessageRequest {
    pub instance_id: InstanceId,
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
pub struct AppendControlEventRequest {
    pub instance_id: InstanceId,
    pub turn_id: Option<String>,
    pub event_id: String,
    pub event_kind: ControlEventKind,
    pub detail: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendLedgerWriteRequest {
    pub instance_id: InstanceId,
    pub turn_id: Option<String>,
    pub write_id: String,
    pub write_kind: LedgerWriteKind,
    pub target_table: String,
    pub target_id: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetRuntimeStateRequest {
    pub instance_id: InstanceId,
    pub active_session_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub pending_input_id: Option<String>,
    pub pending_approval_id: Option<String>,
    pub runtime_status: RuntimeStatus,
    pub dirty_shutdown: bool,
    pub recovery_required: bool,
    pub updated_at: Timestamp,
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
