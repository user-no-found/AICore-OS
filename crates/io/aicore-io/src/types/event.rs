use aicore_foundation::{InstanceId, SessionId, Timestamp};
use aicore_session::{MessageId, TurnId};
use serde::{Deserialize, Serialize};

use super::{
    IoConnectionId, IoDeliveryMode, IoEventId, IoInputKind, IoOutputKind, IoRequestId,
    IoStreamCursor,
};
use crate::types::IoClientId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoInputEnvelope {
    pub input_id: IoRequestId,
    pub input_kind: IoInputKind,
    pub content: Option<String>,
    pub summary_zh: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub redaction_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoOutputEvent {
    pub output_kind: IoOutputKind,
    pub message_id: Option<MessageId>,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<TurnId>,
    pub content: Option<String>,
    pub summary_zh: Option<String>,
    pub summary_en: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub redaction_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoEventEnvelope {
    pub instance_id: InstanceId,
    pub event_id: IoEventId,
    pub client_id: Option<IoClientId>,
    pub connection_id: Option<IoConnectionId>,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<TurnId>,
    pub output: IoOutputEvent,
    pub cursor: IoStreamCursor,
    pub delivery_mode: IoDeliveryMode,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}
