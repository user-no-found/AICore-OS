use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoClientKind {
    Cli,
    Tui,
    Web,
    Gateway,
    Test,
}

impl IoClientKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Tui => "tui",
            Self::Web => "web",
            Self::Gateway => "gateway",
            Self::Test => "test",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoAttachMode {
    Bind,
    Attach,
    Observe,
}

impl IoAttachMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bind => "bind",
            Self::Attach => "attach",
            Self::Observe => "observe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoClientStatus {
    Connecting,
    Attached,
    Detached,
    Stale,
    Closed,
}

impl IoClientStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Connecting => "connecting",
            Self::Attached => "attached",
            Self::Detached => "detached",
            Self::Stale => "stale",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoInputKind {
    UserMessage,
    StopRequest,
    ApprovalResponse,
    PendingInputConfirm,
    PendingInputCancel,
}

impl IoInputKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserMessage => "user_message",
            Self::StopRequest => "stop_request",
            Self::ApprovalResponse => "approval_response",
            Self::PendingInputConfirm => "pending_input_confirm",
            Self::PendingInputCancel => "pending_input_cancel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoOutputKind {
    Snapshot,
    UserMessageCommitted,
    AssistantDelta,
    AssistantFinal,
    ControlEvent,
    ApprovalRequested,
    ApprovalResolved,
    ToolStatus,
    TeamStatus,
    MemoryProposalStatus,
    Error,
}

impl IoOutputKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::UserMessageCommitted => "user_message_committed",
            Self::AssistantDelta => "assistant_delta",
            Self::AssistantFinal => "assistant_final",
            Self::ControlEvent => "control_event",
            Self::ApprovalRequested => "approval_requested",
            Self::ApprovalResolved => "approval_resolved",
            Self::ToolStatus => "tool_status",
            Self::TeamStatus => "team_status",
            Self::MemoryProposalStatus => "memory_proposal_status",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoDeliveryMode {
    Snapshot,
    Replay,
    Live,
}

impl IoDeliveryMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Replay => "replay",
            Self::Live => "live",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoStreamStatus {
    Open,
    CaughtUp,
    Closed,
    StaleCursor,
}

impl IoStreamStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::CaughtUp => "caught_up",
            Self::Closed => "closed",
            Self::StaleCursor => "stale_cursor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoSubmissionStatus {
    Accepted,
    Rejected,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoWriteDisposition {
    NotChecked,
    Applied,
    NotApplied,
}
