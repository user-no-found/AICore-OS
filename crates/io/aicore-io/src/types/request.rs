use aicore_foundation::{InstanceId, SessionId, Timestamp};
use aicore_session::TurnId;
use serde::{Deserialize, Serialize};

use super::{
    IoAttachMode, IoClientId, IoClientKind, IoConnectionId, IoEventId, IoInputEnvelope,
    IoRequestId, IoStreamCursor,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindInstanceRequest {
    pub request_id: IoRequestId,
    pub client_kind: IoClientKind,
    pub attach_mode: IoAttachMode,
    pub instance_id: Option<InstanceId>,
    pub workspace_hint: Option<String>,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachInstanceRequest {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub client_kind: IoClientKind,
    pub attach_mode: IoAttachMode,
    pub from_cursor: Option<IoStreamCursor>,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetachInstanceRequest {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitInputRequest {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<TurnId>,
    pub input: IoInputEnvelope,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopTurnRequest {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<TurnId>,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcknowledgeEventRequest {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub event_id: IoEventId,
    pub cursor: IoStreamCursor,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetCurrentSnapshotRequest {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub session_id: Option<SessionId>,
    pub from_cursor: Option<IoStreamCursor>,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}
