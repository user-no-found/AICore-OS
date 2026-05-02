use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use serde::{Deserialize, Serialize};

use super::{
    SandboxProfileId, ToolApprovalScope, ToolArgsDigest, ToolCallId, ToolCallStatus, ToolId,
    ToolSandboxDecisionKind, ToolSchemaHash, ToolValidationFailureCode,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallEnvelope {
    pub instance_id: InstanceId,
    pub session_id: Option<SessionId>,
    pub turn_id: TurnId,
    pub tool_call_id: ToolCallId,
    pub tool_id: ToolId,
    pub schema_hash: ToolSchemaHash,
    pub args_digest: ToolArgsDigest,
    pub registry_revision: u64,
    pub lock_version: u64,
    pub sandbox_profile_id: SandboxProfileId,
    pub proposed_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolApprovalBinding {
    pub approval_id: String,
    pub approval_scope: ToolApprovalScope,
    pub tool_call_id: ToolCallId,
    pub tool_id: ToolId,
    pub schema_hash: ToolSchemaHash,
    pub args_digest: ToolArgsDigest,
    pub sandbox_profile_id: SandboxProfileId,
    pub lock_version: u64,
    pub target_instance_id: InstanceId,
    pub turn_id: TurnId,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSandboxDecision {
    pub kind: ToolSandboxDecisionKind,
    pub sandbox_profile_id: SandboxProfileId,
    pub reason_code: Option<String>,
}

impl ToolSandboxDecision {
    pub fn allow(sandbox_profile_id: SandboxProfileId) -> Self {
        Self {
            kind: ToolSandboxDecisionKind::Allow,
            sandbox_profile_id,
            reason_code: None,
        }
    }

    pub fn deny(sandbox_profile_id: SandboxProfileId, reason_code: impl Into<String>) -> Self {
        Self {
            kind: ToolSandboxDecisionKind::Deny,
            sandbox_profile_id,
            reason_code: Some(reason_code.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallValidationRequest {
    pub call: ToolCallEnvelope,
    pub computed_args_digest: ToolArgsDigest,
    pub approval_binding: Option<ToolApprovalBinding>,
    pub sandbox_decision: ToolSandboxDecision,
    pub turn_is_active: bool,
    pub stop_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallValidationOutcome {
    pub tool_call_id: ToolCallId,
    pub status: ToolCallStatus,
    pub failure_code: Option<ToolValidationFailureCode>,
    pub execution_summary: Option<ToolExecutionSummary>,
}

impl ToolCallValidationOutcome {
    pub fn failed(tool_call_id: ToolCallId, code: ToolValidationFailureCode) -> Self {
        Self {
            tool_call_id,
            status: ToolCallStatus::ValidationFailed,
            failure_code: Some(code),
            execution_summary: None,
        }
    }

    pub fn approval_required(tool_call_id: ToolCallId) -> Self {
        Self {
            tool_call_id,
            status: ToolCallStatus::ApprovalRequired,
            failure_code: None,
            execution_summary: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionSummary {
    pub tool_call_id: ToolCallId,
    pub status: ToolCallStatus,
    pub summary_en: String,
    pub summary_zh: Option<String>,
    pub did_execute_external_operation: bool,
    pub redaction_applied: bool,
    pub output_truncated: bool,
    pub sensitive_content_omitted: bool,
}
