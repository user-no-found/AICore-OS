use aicore_foundation::{InstanceId, SessionId, Timestamp};
use aicore_session::{MessageId, TurnId};
use serde::{Deserialize, Serialize};

use super::{
    CurrentSnapshot, IoClientId, IoClientStatus, IoConnectionId, IoProtocolError, IoRequestId,
    IoStreamCursor, IoSubmissionStatus, IoSubscriptionId, IoWriteDisposition,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindInstanceResponse {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub subscription_id: IoSubscriptionId,
    pub status: IoClientStatus,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachInstanceResponse {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub subscription_id: IoSubscriptionId,
    pub status: IoClientStatus,
    pub snapshot: CurrentSnapshot,
    pub cursor: IoStreamCursor,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetachInstanceResponse {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub connection_id: IoConnectionId,
    pub status: IoClientStatus,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitInputResponse {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub status: IoSubmissionStatus,
    pub receipt: Option<IoWriteReceipt>,
    pub error: Option<IoProtocolError>,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopTurnResponse {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub status: IoSubmissionStatus,
    pub error: Option<IoProtocolError>,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcknowledgeEventResponse {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub client_id: IoClientId,
    pub cursor: IoStreamCursor,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetCurrentSnapshotResponse {
    pub request_id: IoRequestId,
    pub instance_id: InstanceId,
    pub snapshot: CurrentSnapshot,
    pub cursor: IoStreamCursor,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoWriteReceipt {
    pub write_disposition: IoWriteDisposition,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<TurnId>,
    pub message_id: Option<MessageId>,
    pub idempotency_key: Option<String>,
}
