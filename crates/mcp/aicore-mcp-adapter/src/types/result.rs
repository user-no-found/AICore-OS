use serde::{Deserialize, Serialize};

use super::{McpMappingId, McpRedactionMode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResultSummaryBoundary {
    pub mapping_id: McpMappingId,
    pub redaction_mode: McpRedactionMode,
    pub summary_en: String,
    pub artifact_ref: Option<String>,
    pub redaction_applied: bool,
    pub payload_omitted: bool,
}
