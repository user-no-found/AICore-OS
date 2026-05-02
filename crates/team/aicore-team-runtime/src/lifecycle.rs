use aicore_foundation::Timestamp;
use aicore_team_protocol::*;

pub fn stop_context(
    context: &mut TeamContext,
    channel: &mut TeamChannelState,
    request: TeamStopRequest,
) -> TeamStopOutcome {
    context.status = TeamRunStatus::Stopped;
    channel.status = TeamChannelStatus::Closed;
    channel.closed_at = Some(request.requested_at);
    let mut stopped_agents = 0;
    let mut destroyed_agents = 0;
    for agent in &mut context.agents {
        if matches!(
            agent.status,
            TeamAgentStatus::Running
                | TeamAgentStatus::WaitingTool
                | TeamAgentStatus::WaitingApproval
                | TeamAgentStatus::Created
        ) {
            stopped_agents += 1;
        }
        agent.status = TeamAgentStatus::Destroyed;
        destroyed_agents += 1;
    }
    TeamStopOutcome {
        team_run_id: context.team_run_id.clone(),
        status: context.status,
        stopped_agents,
        destroyed_agents,
        channel_closed: true,
    }
}

pub fn destroy_context(
    context: &mut TeamContext,
    channel: &mut TeamChannelState,
    destroyed_at: Timestamp,
) -> TeamDestroySummary {
    context.status = TeamRunStatus::Destroyed;
    channel.status = TeamChannelStatus::Destroyed;
    channel.closed_at = Some(destroyed_at);
    for agent in &mut context.agents {
        agent.status = TeamAgentStatus::Destroyed;
    }
    TeamDestroySummary {
        team_run_id: context.team_run_id.clone(),
        status: context.status,
        destroyed_agents: context.agents.len(),
        destroyed_at,
    }
}
