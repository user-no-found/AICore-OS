use super::{
    ModelResponseEvent, ModelResponseEventKind, ModelRunId, ModelRunStatus, ModelStopReason,
};
use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelFinalResponse {
    pub run_id: ModelRunId,
    pub status: ModelRunStatus,
    pub final_text: Option<String>,
    pub stop_reason: Option<ModelStopReason>,
    pub committed_event_index: Option<usize>,
}

impl ModelFinalResponse {
    pub fn from_events(
        events: &[ModelResponseEvent],
        stop_requested_at: Option<Timestamp>,
    ) -> Result<Self, String> {
        let Some(first) = events.first() else {
            return Err("model response requires at least one event".to_string());
        };

        if events
            .iter()
            .any(|event| event.kind == ModelResponseEventKind::Cancelled)
        {
            return Ok(Self::new(first, ModelRunStatus::Cancelled, None, None));
        }
        if events
            .iter()
            .any(|event| event.kind == ModelResponseEventKind::ProviderError)
        {
            return Ok(Self::new(first, ModelRunStatus::Failed, None, None));
        }

        for (index, event) in events.iter().enumerate() {
            if event.kind != ModelResponseEventKind::AssistantFinal {
                continue;
            }
            if let Some(stop_at) = stop_requested_at
                && stop_at < event.created_at
            {
                return Ok(Self::new(
                    first,
                    ModelRunStatus::StoppedBeforeFinal,
                    None,
                    Some(ModelStopReason::StopRequested),
                ));
            }
            return Ok(Self {
                run_id: first.run_id.clone(),
                status: ModelRunStatus::Completed,
                final_text: event.text.clone(),
                stop_reason: Some(ModelStopReason::ProviderCompleted),
                committed_event_index: Some(index),
            });
        }

        Ok(Self::new(first, ModelRunStatus::NotFinal, None, None))
    }

    fn new(
        event: &ModelResponseEvent,
        status: ModelRunStatus,
        final_text: Option<String>,
        stop_reason: Option<ModelStopReason>,
    ) -> Self {
        Self {
            run_id: event.run_id.clone(),
            status,
            final_text,
            stop_reason,
            committed_event_index: None,
        }
    }
}
