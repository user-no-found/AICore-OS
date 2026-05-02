use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Installed,
    Enabled,
    Disabled,
    Removed,
    Broken,
    Updating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionClass {
    SafeRead,
    WorkspaceWrite,
    CommandExec,
    Network,
    Privileged,
    Destructive,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolApprovalRequirement {
    NotRequired,
    Required,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolApprovalScope {
    SingleToolCall,
}

impl ToolApprovalScope {
    pub fn from_contract_value(value: &str) -> Option<Self> {
        match value {
            "single_tool_call" => Some(Self::SingleToolCall),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Proposed,
    ValidationFailed,
    ApprovalRequired,
    Approved,
    Rejected,
    ExecutionSkipped,
    CompletedMock,
    FailedMock,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolHotPlugChangeKind {
    Added,
    Enabled,
    Disabled,
    Removed,
    SchemaChanged,
    Broken,
    Repaired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolValidationFailureCode {
    ToolNotFound,
    ToolDisabled,
    ToolRemoved,
    ToolBroken,
    ForbiddenTool,
    SchemaHashMismatch,
    ArgsDigestMismatch,
    LockVersionMismatch,
    ApprovalMissing,
    ApprovalScopeInvalid,
    ApprovalBindingMismatch,
    SandboxDenied,
    TurnStopped,
    TurnStale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSandboxDecisionKind {
    Allow,
    Deny,
    RequiresApproval,
    Forbidden,
}
