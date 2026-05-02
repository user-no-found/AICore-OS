use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

use super::{McpDiscoveryId, McpServerId, McpServerStatus, McpToolCandidate, McpTrustLevel};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerDescriptor {
    pub server_id: McpServerId,
    pub display_name: String,
    pub status: McpServerStatus,
    pub trust_level: McpTrustLevel,
    pub discovery_id: McpDiscoveryId,
    pub summary_en: String,
    pub configured_by_user: bool,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpDiscoveryReport {
    pub discovery_id: McpDiscoveryId,
    pub servers: Vec<McpServerDescriptor>,
    pub tool_candidates: Vec<McpToolCandidate>,
    pub created_at: Timestamp,
}
