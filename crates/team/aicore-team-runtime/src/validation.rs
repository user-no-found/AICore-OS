use aicore_team_protocol::*;

pub fn validate_spawn_request(
    context: &TeamContext,
    channel: &TeamChannelState,
    policy: &TeamPolicy,
    request: &TeamSpawnRequest,
) -> Result<(), TeamSpawnFailureCode> {
    if context.status != TeamRunStatus::Running || context.parent_turn_id != request.parent_turn_id
    {
        return Err(TeamSpawnFailureCode::TurnNotActive);
    }
    if channel.status != TeamChannelStatus::Open {
        return Err(TeamSpawnFailureCode::ChannelClosed);
    }
    if request.spawn_depth > policy.max_spawn_depth {
        return Err(TeamSpawnFailureCode::SpawnDepthExceeded);
    }
    if context.agents.len() >= policy.max_team_agents_per_turn {
        return Err(TeamSpawnFailureCode::TooManyAgents);
    }
    let running = context
        .agents
        .iter()
        .filter(|agent| agent.status == TeamAgentStatus::Running)
        .count();
    if running >= policy.max_concurrent_team_agents {
        return Err(TeamSpawnFailureCode::ConcurrencyLimit);
    }
    if !policy
        .allowed_models
        .iter()
        .any(|model| model == &request.model)
    {
        return Err(TeamSpawnFailureCode::InvalidModel);
    }
    if request.budget.is_none() {
        return Err(TeamSpawnFailureCode::BudgetMissing);
    }
    if request
        .allowed_tools
        .iter()
        .any(|tool| !policy.tool_snapshot.iter().any(|allowed| allowed == tool))
    {
        return Err(TeamSpawnFailureCode::ToolNotAllowed);
    }
    if let (Some(deadline), Some(parent_deadline)) = (request.deadline, policy.parent_deadline) {
        if deadline > parent_deadline {
            return Err(TeamSpawnFailureCode::TurnNotActive);
        }
    }
    Ok(())
}

pub fn descriptor_from_request(request: &TeamSpawnRequest) -> TeamAgentDescriptor {
    TeamAgentDescriptor {
        team_agent_id: request.team_agent_id.clone(),
        role_name: request.role_name.clone(),
        task: request.task.clone(),
        model: request.model.clone(),
        allowed_tools: request.allowed_tools.clone(),
        status: TeamAgentStatus::Running,
        spawn_depth: request.spawn_depth,
        created_at: request.created_at,
    }
}
