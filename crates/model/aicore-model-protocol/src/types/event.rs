use super::{ModelEventId, ModelRequestEnvelope, ModelResponseEventKind, ModelRunId};
use aicore_foundation::{InstanceId, SessionId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelResponseEvent {
    pub event_id: ModelEventId,
    pub run_id: ModelRunId,
    pub instance_id: InstanceId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<String>,
    pub kind: ModelResponseEventKind,
    pub text: Option<String>,
    pub error_code: Option<String>,
    pub usage: Option<ModelUsageSummary>,
    pub created_at: Timestamp,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelUsageSummary {
    pub input_units: u64,
    pub output_units: u64,
}

impl ModelResponseEvent {
    pub fn started(request: &ModelRequestEnvelope, created_at: Timestamp) -> Self {
        Self::from_request(
            request,
            ModelResponseEventKind::RequestStarted,
            None,
            None,
            created_at,
        )
    }

    pub fn assistant_delta(
        request: &ModelRequestEnvelope,
        text: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self::from_request(
            request,
            ModelResponseEventKind::AssistantDelta,
            Some(text.into()),
            None,
            created_at,
        )
    }

    pub fn assistant_final(
        request: &ModelRequestEnvelope,
        text: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self::from_request(
            request,
            ModelResponseEventKind::AssistantFinal,
            Some(text.into()),
            None,
            created_at,
        )
    }

    pub fn provider_error(
        request: &ModelRequestEnvelope,
        code: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self::from_request(
            request,
            ModelResponseEventKind::ProviderError,
            None,
            Some(code.into()),
            created_at,
        )
    }

    pub fn cancelled(request: &ModelRequestEnvelope, created_at: Timestamp) -> Self {
        Self::from_request(
            request,
            ModelResponseEventKind::Cancelled,
            None,
            None,
            created_at,
        )
    }

    pub fn stopped_before_final(request: &ModelRequestEnvelope, created_at: Timestamp) -> Self {
        Self::from_request(
            request,
            ModelResponseEventKind::StoppedBeforeFinal,
            None,
            None,
            created_at,
        )
    }

    pub fn completed(request: &ModelRequestEnvelope, created_at: Timestamp) -> Self {
        Self::from_request(
            request,
            ModelResponseEventKind::Completed,
            None,
            None,
            created_at,
        )
    }

    fn from_request(
        request: &ModelRequestEnvelope,
        kind: ModelResponseEventKind,
        text: Option<String>,
        error_code: Option<String>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            event_id: ModelEventId::new(format!("model.event.{}", created_at.unix_millis()))
                .expect("generated model event id should be valid"),
            run_id: request.run_id.clone(),
            instance_id: request.instance_id.clone(),
            session_id: request.session_id.clone(),
            turn_id: request.turn_id.clone(),
            kind,
            text,
            error_code,
            usage: None,
            created_at,
            correlation_id: request.correlation_id.clone(),
            causation_id: Some(request.request_id.as_str().to_string()),
        }
    }
}
