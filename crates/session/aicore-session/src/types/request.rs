use super::enums::{
    ActiveTurnAcquireStatus, ApprovalDecision, ApprovalResponseStatus, ApprovalScope,
    ApprovalStatus, ControlEventKind, LedgerWriteKind, MessageKind, PendingInputStatus,
    RuntimeStatus, SessionStatus, StopTurnStatus, TurnStatus,
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
    pub lock_version: Option<u64>,
    pub dirty_shutdown: bool,
    pub recovery_required: bool,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTurnAcquireRequest {
    pub instance_id: InstanceId,
    pub session_id: SessionId,
    pub turn_id: String,
    pub requested_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTurnAcquireOutcome {
    pub status: ActiveTurnAcquireStatus,
    pub active_turn_id: Option<String>,
    pub lock_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTurnReleaseRequest {
    pub instance_id: InstanceId,
    pub turn_id: String,
    pub terminal_status: TurnStatus,
    pub released_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTurnReleaseOutcome {
    pub released: bool,
    pub lock_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingInputSubmitRequest {
    pub instance_id: InstanceId,
    pub pending_input_id: String,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub content: String,
    pub submitted_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingInputSubmitOutcome {
    pub pending_input_id: String,
    pub replaced_pending_input_id: Option<String>,
    pub status: PendingInputStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingInputCancelRequest {
    pub instance_id: InstanceId,
    pub cancelled_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingInputCancelOutcome {
    pub cancelled_pending_input_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopTurnRequest {
    pub instance_id: InstanceId,
    pub requested_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopTurnOutcome {
    pub status: StopTurnStatus,
    pub turn_id: Option<String>,
    pub lock_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateApprovalRequest {
    pub instance_id: InstanceId,
    pub approval_id: String,
    pub turn_id: String,
    pub scope: ApprovalScope,
    pub summary: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalResponseRequest {
    pub instance_id: InstanceId,
    pub approval_id: String,
    pub response_id: String,
    pub decision: ApprovalDecision,
    pub responder_client_id: Option<String>,
    pub responder_client_kind: Option<String>,
    pub responded_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalResponseOutcome {
    pub response_id: String,
    pub status: ApprovalResponseStatus,
    pub approval_status: ApprovalStatus,
    pub resolved_response_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidateApprovalsRequest {
    pub instance_id: InstanceId,
    pub turn_id: String,
    pub status: ApprovalStatus,
    pub invalidated_at: Timestamp,
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
