use aicore_foundation::Timestamp;
use aicore_model_protocol::ModelId;
use aicore_tool_protocol::ToolId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamBudget {
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub max_messages: u32,
    pub deadline_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamPolicy {
    pub max_team_agents_per_turn: usize,
    pub max_concurrent_team_agents: usize,
    pub max_spawn_depth: u8,
    pub allowed_models: Vec<ModelId>,
    pub tool_snapshot: Vec<ToolId>,
    pub parent_deadline: Option<Timestamp>,
}

impl TeamPolicy {
    pub fn default_mock(allowed_models: Vec<ModelId>, tool_snapshot: Vec<ToolId>) -> Self {
        Self {
            max_team_agents_per_turn: 4,
            max_concurrent_team_agents: 2,
            max_spawn_depth: 1,
            allowed_models,
            tool_snapshot,
            parent_deadline: None,
        }
    }
}
