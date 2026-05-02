use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

use super::{McpServerId, McpToolCandidateStatus, McpToolName};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolCandidate {
    pub server_id: McpServerId,
    pub tool_name: McpToolName,
    pub status: McpToolCandidateStatus,
    pub summary_en: String,
    pub input_schema_summary: String,
    pub output_schema_summary: String,
    pub prompt_visible: bool,
    pub executable: bool,
    pub discovered_at: Timestamp,
}

impl McpToolCandidate {
    pub fn discovered(
        server_id: McpServerId,
        tool_name: McpToolName,
        summary_en: String,
        discovered_at: Timestamp,
    ) -> Self {
        Self {
            server_id,
            tool_name,
            status: McpToolCandidateStatus::Discovered,
            summary_en,
            input_schema_summary: "candidate input summary unavailable".to_string(),
            output_schema_summary: "candidate output summary unavailable".to_string(),
            prompt_visible: false,
            executable: false,
            discovered_at,
        }
    }
}
