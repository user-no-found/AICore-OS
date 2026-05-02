use crate::{ModelProvider, ModelRequestEnvelope, ModelResponseEvent, ModelResponseEventKind};
use aicore_foundation::Timestamp;

#[derive(Debug, Clone)]
pub struct ScriptedModelProvider {
    script: Vec<ModelResponseEventKind>,
}

impl ScriptedModelProvider {
    pub fn new(script: Vec<ModelResponseEventKind>) -> Self {
        Self { script }
    }
}

impl ModelProvider for ScriptedModelProvider {
    fn invoke(&self, request: &ModelRequestEnvelope) -> Result<Vec<ModelResponseEvent>, String> {
        let mut events = Vec::with_capacity(self.script.len());
        for (index, kind) in self.script.iter().copied().enumerate() {
            let created_at = Timestamp::from_unix_millis(index as u128 + 1);
            let event = match kind {
                ModelResponseEventKind::RequestStarted => {
                    ModelResponseEvent::started(request, created_at)
                }
                ModelResponseEventKind::AssistantDelta => {
                    ModelResponseEvent::assistant_delta(request, "scripted delta", created_at)
                }
                ModelResponseEventKind::AssistantFinal => {
                    ModelResponseEvent::assistant_final(request, "scripted final", created_at)
                }
                ModelResponseEventKind::ProviderError => {
                    ModelResponseEvent::provider_error(request, "scripted_error", created_at)
                }
                ModelResponseEventKind::Cancelled => {
                    ModelResponseEvent::cancelled(request, created_at)
                }
                ModelResponseEventKind::StoppedBeforeFinal => {
                    ModelResponseEvent::stopped_before_final(request, created_at)
                }
                ModelResponseEventKind::Completed => {
                    ModelResponseEvent::completed(request, created_at)
                }
            };
            events.push(event);
        }
        Ok(events)
    }
}
