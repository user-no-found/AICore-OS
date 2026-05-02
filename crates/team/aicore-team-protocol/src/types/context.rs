use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use serde::{Deserialize, Serialize};

use super::{
    TeamAgentDescriptor, TeamAgentId, TeamBudget, TeamChannelId, TeamRunId, TeamRunStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamContext {
    pub parent_instance_id: InstanceId,
    pub parent_session_id: SessionId,
    pub parent_turn_id: TurnId,
    pub team_run_id: TeamRunId,
    pub team_channel_id: TeamChannelId,
    pub team_generation: u64,
    pub created_by_agent_id: TeamAgentId,
    pub created_at: Timestamp,
    pub status: TeamRunStatus,
    pub team_budget: TeamBudget,
    pub spawn_depth_limit: u8,
    pub concurrency_limit: usize,
    pub agents: Vec<TeamAgentDescriptor>,
}
