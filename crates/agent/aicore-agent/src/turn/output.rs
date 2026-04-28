use crate::turn::AgentTurnFailureStage;
use crate::turn::input::AgentTurnDebug;
use crate::turn::surface::{ConversationSurface, TurnSurfaceEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTurnOutcome {
    Completed,
    Queued,
    AppendedContext,
    Interrupted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnOutput {
    pub assistant_output: Option<String>,
    pub memory_count: usize,
    pub provider_name: Option<String>,
    pub provider_kind: Option<String>,
    pub prompt_builder_ok: bool,
    pub runtime_output_ok: bool,
    pub provider_invoked: bool,
    pub assistant_output_generated: bool,
    pub outcome: AgentTurnOutcome,
    pub error_message: Option<String>,
    pub failure_stage: Option<AgentTurnFailureStage>,
    pub accepted_source: String,
    pub ingress_decision: String,
    pub conversation_id: String,
    pub active_turn_id: Option<String>,
    pub active_turn_status: Option<String>,
    pub conversation_status: String,
    pub event_count: usize,
    pub queue_len: usize,
    pub debug: Option<AgentTurnDebug>,
}

impl AgentTurnOutput {
    pub fn to_surface_entry(&self) -> TurnSurfaceEntry {
        TurnSurfaceEntry {
            conversation_id: self.conversation_id.clone(),
            turn_id: self.active_turn_id.clone(),
            accepted_source: self.accepted_source.clone(),
            ingress_decision: self.ingress_decision.clone(),
            outcome: self.outcome.clone(),
            conversation_status: self.conversation_status.clone(),
            active_turn_status: self.active_turn_status.clone(),
            queue_len: self.queue_len,
            event_count: self.event_count,
            memory_count: self.memory_count,
            assistant_output_present: self.assistant_output.is_some(),
            provider_invoked: self.provider_invoked,
            provider_kind: self.provider_kind.clone(),
            provider_name: self.provider_name.clone(),
            failure_stage: self.failure_stage.clone(),
            error_message: self.error_message.clone(),
        }
    }

    pub fn to_conversation_surface(&self) -> ConversationSurface {
        ConversationSurface {
            conversation_id: self.conversation_id.clone(),
            latest_turn: self.to_surface_entry(),
        }
    }
}
