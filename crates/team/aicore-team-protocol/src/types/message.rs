use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

use super::{
    TeamAgentId, TeamChannelId, TeamChannelStatus, TeamCommunicationScope, TeamMessageId,
    TeamMessageKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamChannelState {
    pub team_channel_id: TeamChannelId,
    pub status: TeamChannelStatus,
    pub created_at: Timestamp,
    pub closed_at: Option<Timestamp>,
    pub message_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamMessage {
    pub message_id: TeamMessageId,
    pub team_channel_id: TeamChannelId,
    pub sender_agent_id: TeamAgentId,
    pub recipient_agent_id: Option<TeamAgentId>,
    pub kind: TeamMessageKind,
    pub communication_scope: TeamCommunicationScope,
    pub summary_en: String,
    pub summary_zh: Option<String>,
    pub source_refs: Vec<String>,
    pub created_at: Timestamp,
    pub seq: u64,
}
