use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use aicore_team_protocol::*;

use crate::{channel::append_message_to_channel, lifecycle, validation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamRuntimeError {
    ContextAlreadyExists,
    MissingContext,
    ChannelClosed,
    Destroyed,
    AgentNotFound,
}

#[derive(Debug, Clone)]
pub struct InMemoryTeamRuntime {
    policy: TeamPolicy,
    context: Option<TeamContext>,
    channel: Option<TeamChannelState>,
    messages: Vec<TeamMessage>,
    results: Vec<TeamResult>,
}

impl InMemoryTeamRuntime {
    pub fn new(policy: TeamPolicy) -> Self {
        Self {
            policy,
            context: None,
            channel: None,
            messages: Vec::new(),
            results: Vec::new(),
        }
    }

    pub fn create_team_context(
        &mut self,
        parent_instance_id: InstanceId,
        parent_session_id: SessionId,
        parent_turn_id: TurnId,
        team_run_id: TeamRunId,
        team_channel_id: TeamChannelId,
        created_by_agent_id: TeamAgentId,
        team_budget: TeamBudget,
        created_at: Timestamp,
    ) -> Result<TeamContext, TeamRuntimeError> {
        if self.context.is_some() {
            return Err(TeamRuntimeError::ContextAlreadyExists);
        }
        let context = TeamContext {
            parent_instance_id,
            parent_session_id,
            parent_turn_id,
            team_run_id,
            team_channel_id: team_channel_id.clone(),
            team_generation: 1,
            created_by_agent_id,
            created_at,
            status: TeamRunStatus::Running,
            team_budget,
            spawn_depth_limit: self.policy.max_spawn_depth,
            concurrency_limit: self.policy.max_concurrent_team_agents,
            agents: Vec::new(),
        };
        let channel = TeamChannelState {
            team_channel_id,
            status: TeamChannelStatus::Open,
            created_at,
            closed_at: None,
            message_seq: 0,
        };
        self.context = Some(context.clone());
        self.channel = Some(channel);
        Ok(context)
    }

    pub fn spawn_team_agent(
        &mut self,
        request: TeamSpawnRequest,
    ) -> Result<TeamSpawnOutcome, TeamSpawnFailureCode> {
        let context = self
            .context
            .as_mut()
            .ok_or(TeamSpawnFailureCode::TurnNotActive)?;
        let channel = self
            .channel
            .as_ref()
            .ok_or(TeamSpawnFailureCode::ChannelClosed)?;
        if context.status == TeamRunStatus::Destroyed {
            return Err(TeamSpawnFailureCode::ChannelClosed);
        }
        validation::validate_spawn_request(context, channel, &self.policy, &request)?;
        let agent = validation::descriptor_from_request(&request);
        context.agents.push(agent.clone());
        Ok(TeamSpawnOutcome {
            agent,
            failure_code: None,
        })
    }

    pub fn append_team_message(
        &mut self,
        message: TeamMessage,
    ) -> Result<TeamMessage, TeamRuntimeError> {
        let context = self
            .context
            .as_ref()
            .ok_or(TeamRuntimeError::MissingContext)?;
        if context.status == TeamRunStatus::Destroyed {
            return Err(TeamRuntimeError::Destroyed);
        }
        let channel = self
            .channel
            .as_mut()
            .ok_or(TeamRuntimeError::ChannelClosed)?;
        append_message_to_channel(channel, &mut self.messages, message)
    }

    pub fn submit_team_result(
        &mut self,
        mut result: TeamResult,
    ) -> Result<TeamResult, TeamRuntimeError> {
        let context = self
            .context
            .as_ref()
            .ok_or(TeamRuntimeError::MissingContext)?;
        let channel = self
            .channel
            .as_ref()
            .ok_or(TeamRuntimeError::ChannelClosed)?;
        if context.status == TeamRunStatus::Destroyed {
            result.status = TeamResultStatus::RejectedChannelClosed;
            return Ok(result);
        }
        if context.status == TeamRunStatus::Stopped {
            result.status = TeamResultStatus::RejectedTurnStopped;
            return Ok(result);
        }
        if channel.status != TeamChannelStatus::Open {
            result.status = TeamResultStatus::RejectedChannelClosed;
            return Ok(result);
        }
        result.status = TeamResultStatus::Accepted;
        self.results.push(result.clone());
        Ok(result)
    }

    pub fn stop_team_run(
        &mut self,
        request: TeamStopRequest,
    ) -> Result<TeamStopOutcome, TeamRuntimeError> {
        let context = self
            .context
            .as_mut()
            .ok_or(TeamRuntimeError::MissingContext)?;
        let channel = self
            .channel
            .as_mut()
            .ok_or(TeamRuntimeError::ChannelClosed)?;
        Ok(lifecycle::stop_context(context, channel, request))
    }

    pub fn destroy_team_run(
        &mut self,
        destroyed_at: Timestamp,
    ) -> Result<TeamDestroySummary, TeamRuntimeError> {
        let context = self
            .context
            .as_mut()
            .ok_or(TeamRuntimeError::MissingContext)?;
        let channel = self
            .channel
            .as_mut()
            .ok_or(TeamRuntimeError::ChannelClosed)?;
        Ok(lifecycle::destroy_context(context, channel, destroyed_at))
    }

    pub fn get_team_context(&self) -> Option<&TeamContext> {
        self.context.as_ref()
    }

    pub fn channel_state(&self) -> &TeamChannelState {
        self.channel
            .as_ref()
            .expect("team channel exists after context creation")
    }

    pub fn list_team_messages(&self) -> &[TeamMessage] {
        &self.messages
    }

    pub fn list_team_results(&self) -> &[TeamResult] {
        &self.results
    }

    pub fn mark_agent_completed(&mut self, agent_id: &TeamAgentId) -> Result<(), TeamRuntimeError> {
        let context = self
            .context
            .as_mut()
            .ok_or(TeamRuntimeError::MissingContext)?;
        let Some(agent) = context
            .agents
            .iter_mut()
            .find(|agent| &agent.team_agent_id == agent_id)
        else {
            return Err(TeamRuntimeError::AgentNotFound);
        };
        agent.status = TeamAgentStatus::Completed;
        Ok(())
    }
}

pub fn exported_runtime_symbols() -> &'static [&'static str] {
    &[
        "InMemoryTeamRuntime",
        "create_team_context",
        "spawn_team_agent",
        "append_team_message",
        "submit_team_result",
        "stop_team_run",
        "destroy_team_run",
        "get_team_context",
        "list_team_messages",
        "list_team_results",
    ]
}
