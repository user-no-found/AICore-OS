use crate::turn::{AgentTurnFailureStage, AgentTurnOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSurfaceEntry {
    pub conversation_id: String,
    pub turn_id: Option<String>,
    pub accepted_source: String,
    pub ingress_decision: String,
    pub outcome: AgentTurnOutcome,
    pub conversation_status: String,
    pub active_turn_status: Option<String>,
    pub queue_len: usize,
    pub event_count: usize,
    pub memory_count: usize,
    pub assistant_output_present: bool,
    pub provider_invoked: bool,
    pub provider_kind: Option<String>,
    pub provider_name: Option<String>,
    pub failure_stage: Option<AgentTurnFailureStage>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSurface {
    pub conversation_id: String,
    pub latest_turn: TurnSurfaceEntry,
}
