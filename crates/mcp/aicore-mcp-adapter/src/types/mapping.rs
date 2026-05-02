use aicore_foundation::Timestamp;
use aicore_tool_protocol::{
    SandboxProfileId, ToolApprovalRequirement, ToolId, ToolPermissionClass, ToolSchemaHash,
};
use serde::{Deserialize, Serialize};

use super::{
    McpMappingId, McpMappingStatus, McpMappingValidationOutcome, McpServerDescriptor,
    McpToolCandidate, McpToolCandidateStatus, McpTrustLevel,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolToAicoreToolMapping {
    pub mapping_id: McpMappingId,
    pub server: McpServerDescriptor,
    pub candidate: McpToolCandidate,
    pub tool_id: ToolId,
    pub schema_hash: ToolSchemaHash,
    pub sandbox_profile_id: SandboxProfileId,
    pub status: McpMappingStatus,
    pub permission_class: ToolPermissionClass,
    pub approval_requirement: ToolApprovalRequirement,
    pub registered_in_runtime: bool,
    pub created_at: Timestamp,
}

impl McpToolToAicoreToolMapping {
    pub fn new_candidate(
        mapping_id: McpMappingId,
        server: McpServerDescriptor,
        candidate: McpToolCandidate,
        tool_id: ToolId,
        schema_hash: ToolSchemaHash,
        sandbox_profile_id: SandboxProfileId,
        created_at: Timestamp,
    ) -> Result<Self, McpMappingValidationOutcome> {
        match validate_mcp_tool_mapping(Some(&candidate), &server) {
            McpMappingValidationOutcome::AllowedMappingCandidate => Ok(Self {
                mapping_id,
                server,
                candidate,
                tool_id,
                schema_hash,
                sandbox_profile_id,
                status: McpMappingStatus::RegisteredToolModule,
                permission_class: ToolPermissionClass::SafeRead,
                approval_requirement: ToolApprovalRequirement::Required,
                registered_in_runtime: false,
                created_at,
            }),
            outcome => Err(outcome),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpPermissionHook {
    pub mapping_id: McpMappingId,
    pub requires_tool_registry_registration: bool,
    pub requires_approval: bool,
    pub requires_sandbox: bool,
    pub summary_en: String,
}

pub fn validate_mcp_tool_mapping(
    candidate: Option<&McpToolCandidate>,
    server: &McpServerDescriptor,
) -> McpMappingValidationOutcome {
    let Some(candidate) = candidate else {
        return McpMappingValidationOutcome::RejectedUnknownTool;
    };
    if server.trust_level == McpTrustLevel::Untrusted {
        return McpMappingValidationOutcome::RejectedUntrustedServer;
    }
    if candidate.status != McpToolCandidateStatus::Discovered {
        return McpMappingValidationOutcome::RejectedCandidateNotDiscovered;
    }
    McpMappingValidationOutcome::AllowedMappingCandidate
}
