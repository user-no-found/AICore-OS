use aicore_foundation::{InstanceId, SessionId, Timestamp};
use aicore_session::{ApprovalId, PendingInputId, TurnId};
use serde::{Deserialize, Serialize};

use super::{IoClientId, IoClientKind, IoClientStatus, IoStreamCursor, IoStreamStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentSnapshot {
    pub instance_id: InstanceId,
    pub session_id: Option<SessionId>,
    pub active_turn_id: Option<TurnId>,
    pub visible_turns: Vec<VisibleTurnSummary>,
    pub connected_clients: Vec<VisibleClientSummary>,
    pub pending_input: Option<VisiblePendingInputSummary>,
    pub pending_approval: Option<VisibleApprovalSummary>,
    pub recent_message_cursor: Option<IoStreamCursor>,
    pub stream_status: IoStreamStatus,
    pub recovery_notice: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleTurnSummary {
    pub turn_id: TurnId,
    pub status: String,
    pub summary_zh: Option<String>,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleApprovalSummary {
    pub approval_id: ApprovalId,
    pub turn_id: Option<TurnId>,
    pub summary_zh: Option<String>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisiblePendingInputSummary {
    pub pending_input_id: PendingInputId,
    pub summary_zh: Option<String>,
    pub created_at: Timestamp,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleClientSummary {
    pub client_id: IoClientId,
    pub client_kind: IoClientKind,
    pub status: IoClientStatus,
    pub connected_at: Timestamp,
    pub last_seen_at: Timestamp,
}
