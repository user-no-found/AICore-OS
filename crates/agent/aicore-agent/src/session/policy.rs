use crate::turn::AgentTurnOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSessionContinuationPolicy {
    ContinueAll,
    StopOnFailed,
    StopOnNonCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSessionStopReason {
    Failed,
    Queued,
    AppendedContext,
    Interrupted,
}

pub(crate) fn session_stop_reason(
    policy: &AgentSessionContinuationPolicy,
    outcome: &AgentTurnOutcome,
) -> Option<AgentSessionStopReason> {
    match policy {
        AgentSessionContinuationPolicy::ContinueAll => None,
        AgentSessionContinuationPolicy::StopOnFailed => match outcome {
            AgentTurnOutcome::Failed => Some(AgentSessionStopReason::Failed),
            _ => None,
        },
        AgentSessionContinuationPolicy::StopOnNonCompleted => match outcome {
            AgentTurnOutcome::Completed => None,
            AgentTurnOutcome::Failed => Some(AgentSessionStopReason::Failed),
            AgentTurnOutcome::Queued => Some(AgentSessionStopReason::Queued),
            AgentTurnOutcome::AppendedContext => Some(AgentSessionStopReason::AppendedContext),
            AgentTurnOutcome::Interrupted => Some(AgentSessionStopReason::Interrupted),
        },
    }
}
