use aicore_kernel::InstanceRuntime;

use crate::names::conversation_status_name;
use crate::session::AgentSessionStopReason;
use crate::turn::{AgentTurnOutput, TurnSurfaceEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionSurface {
    pub conversation_id: String,
    pub turn_count: usize,
    pub latest_turn: Option<TurnSurfaceEntry>,
    pub turns: Vec<TurnSurfaceEntry>,
    pub event_count: usize,
    pub queue_len: usize,
    pub conversation_status: String,
    pub completed_all_inputs: bool,
    pub stop_reason: Option<AgentSessionStopReason>,
}

pub(crate) fn session_surface_from_outputs(
    runtime: &InstanceRuntime,
    turn_outputs: &[AgentTurnOutput],
    completed_all_inputs: bool,
    stop_reason: Option<AgentSessionStopReason>,
) -> AgentSessionSurface {
    let runtime_summary = runtime.summary();
    let turns = turn_outputs
        .iter()
        .map(AgentTurnOutput::to_surface_entry)
        .collect::<Vec<_>>();
    AgentSessionSurface {
        conversation_id: runtime_summary.conversation_id,
        turn_count: turns.len(),
        latest_turn: turns.last().cloned(),
        turns,
        event_count: runtime_summary.event_count,
        queue_len: runtime.turn_state().queue_len,
        conversation_status: conversation_status_name(&runtime_summary.conversation_status)
            .to_string(),
        completed_all_inputs,
        stop_reason,
    }
}
