use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

use super::{
    TeamAgentId, TeamMessageKind, TeamResultId, TeamResultStatus, TeamRunId, TeamRunStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamFinding {
    pub kind: TeamMessageKind,
    pub summary_en: String,
    pub summary_zh: Option<String>,
    pub source_refs: Vec<String>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamResult {
    pub result_id: TeamResultId,
    pub team_agent_id: TeamAgentId,
    pub status: TeamResultStatus,
    pub summary_en: String,
    pub summary_zh: Option<String>,
    pub findings: Vec<TeamFinding>,
    pub risks: Vec<String>,
    pub confidence: u8,
    pub source_refs: Vec<String>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamStopRequest {
    pub team_run_id: TeamRunId,
    pub requested_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamStopOutcome {
    pub team_run_id: TeamRunId,
    pub status: TeamRunStatus,
    pub stopped_agents: usize,
    pub destroyed_agents: usize,
    pub channel_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamDestroySummary {
    pub team_run_id: TeamRunId,
    pub status: TeamRunStatus,
    pub destroyed_agents: usize,
    pub destroyed_at: Timestamp,
}
