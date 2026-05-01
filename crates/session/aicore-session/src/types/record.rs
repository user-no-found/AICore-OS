use super::enums::{
    ControlEventType, LedgerWriteType, MessageKind, RuntimeStatus, SessionStatus, TurnStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub title: String,
    pub status: SessionStatus,
    pub created_at: u128,
    pub updated_at: u128,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRecord {
    pub turn_id: String,
    pub session_id: String,
    pub turn_seq: u64,
    pub status: TurnStatus,
    pub started_at: u128,
    pub finished_at: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRecord {
    pub message_id: String,
    pub session_id: String,
    pub turn_id: Option<String>,
    pub message_seq: u64,
    pub kind: MessageKind,
    pub content: String,
    pub created_at: u128,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlEvent {
    pub event_id: String,
    pub instance_id: String,
    pub turn_id: Option<String>,
    pub event_seq: u64,
    pub event_type: ControlEventType,
    pub detail: String,
    pub created_at: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerWrite {
    pub write_id: String,
    pub instance_id: String,
    pub turn_id: Option<String>,
    pub write_seq: u64,
    pub write_type: LedgerWriteType,
    pub target_table: String,
    pub target_id: String,
    pub created_at: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceRuntimeState {
    pub instance_id: String,
    pub active_session_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub pending_input_id: Option<String>,
    pub pending_approval_id: Option<String>,
    pub last_message_seq: Option<u64>,
    pub last_control_event_seq: Option<u64>,
    pub last_write_seq: Option<u64>,
    pub runtime_status: RuntimeStatus,
    pub dirty_shutdown: bool,
    pub recovery_required: bool,
    pub updated_at: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceRuntimeSnapshot {
    pub instance_id: String,
    pub active_session_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub last_message_seq: Option<u64>,
    pub runtime_status: RuntimeStatus,
    pub dirty_shutdown: bool,
    pub recovery_required: bool,
}
