use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use aicore_model_protocol::ModelId;
use aicore_tool_protocol::ToolId;
use serde::{Deserialize, Serialize};

use super::{
    TeamAgentId, TeamAgentStatus, TeamBudget, TeamChannelId, TeamCommunicationScope,
    TeamSpawnFailureCode,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamAgentDescriptor {
    pub team_agent_id: TeamAgentId,
    pub role_name: String,
    pub task: String,
    pub model: ModelId,
    pub allowed_tools: Vec<ToolId>,
    pub status: TeamAgentStatus,
    pub spawn_depth: u8,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSpawnRequest {
    pub team_agent_id: TeamAgentId,
    pub parent_instance_id: InstanceId,
    pub parent_session_id: SessionId,
    pub parent_turn_id: TurnId,
    pub team_channel_id: TeamChannelId,
    pub role_name: String,
    pub task: String,
    pub model: ModelId,
    pub instructions: String,
    pub allowed_tools: Vec<ToolId>,
    pub communication_scope: TeamCommunicationScope,
    pub output_contract: String,
    pub budget: Option<TeamBudget>,
    pub deadline: Option<Timestamp>,
    pub spawn_depth: u8,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSpawnOutcome {
    pub agent: TeamAgentDescriptor,
    pub failure_code: Option<TeamSpawnFailureCode>,
}
