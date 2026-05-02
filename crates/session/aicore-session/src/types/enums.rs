use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Archived,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Active,
    Running,
    WaitingApproval,
    Stopping,
    Stopped,
    Completed,
    Interrupted,
    Cancelled,
    Failed,
    InterruptedByRecovery,
}

impl TurnStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::InterruptedByRecovery => "interrupted_by_recovery",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Stopped
                | Self::Completed
                | Self::Interrupted
                | Self::Cancelled
                | Self::Failed
                | Self::InterruptedByRecovery
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    User,
    AssistantDelta,
    AssistantFinal,
    System,
    ToolCall,
    ToolResult,
}

impl MessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::AssistantDelta => "assistant_delta",
            Self::AssistantFinal => "assistant_final",
            Self::System => "system",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlEventKind {
    SessionCreated,
    TurnBegan,
    TurnFinished,
    MessageAppended,
    TurnInterrupted,
    RuntimeStateUpdated,
    ActiveTurnAcquired,
    ActiveTurnReleased,
    StopRequested,
    PendingInputSubmitted,
    PendingInputCancelled,
    ApprovalCreated,
    ApprovalResolved,
    ApprovalInvalidated,
    Custom,
}

impl ControlEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SessionCreated => "session_created",
            Self::TurnBegan => "turn_began",
            Self::TurnFinished => "turn_finished",
            Self::MessageAppended => "message_appended",
            Self::TurnInterrupted => "turn_interrupted",
            Self::RuntimeStateUpdated => "runtime_state_updated",
            Self::ActiveTurnAcquired => "active_turn_acquired",
            Self::ActiveTurnReleased => "active_turn_released",
            Self::StopRequested => "stop_requested",
            Self::PendingInputSubmitted => "pending_input_submitted",
            Self::PendingInputCancelled => "pending_input_cancelled",
            Self::ApprovalCreated => "approval_created",
            Self::ApprovalResolved => "approval_resolved",
            Self::ApprovalInvalidated => "approval_invalidated",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerWriteKind {
    Insert,
    Update,
    Delete,
}

impl LedgerWriteKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

pub type ControlEventType = ControlEventKind;
pub type LedgerWriteType = LedgerWriteKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
    Expired,
    Stale,
    InvalidatedByStop,
    InvalidatedByTurnClose,
    InvalidatedByRecovery,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Stale => "stale",
            Self::InvalidatedByStop => "invalidated_by_stop",
            Self::InvalidatedByTurnClose => "invalidated_by_turn_close",
            Self::InvalidatedByRecovery => "invalidated_by_recovery",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingInputStatus {
    Pending,
    Confirmed,
    Cancelled,
    Replaced,
    Expired,
    Stale,
}

impl PendingInputStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
            Self::Cancelled => "cancelled",
            Self::Replaced => "replaced",
            Self::Expired => "expired",
            Self::Stale => "stale",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveTurnAcquireStatus {
    Acquired,
    AlreadyActive,
    Rejected,
}

impl ActiveTurnAcquireStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Acquired => "acquired",
            Self::AlreadyActive => "already_active",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopTurnStatus {
    StopRequested,
    AlreadyTerminal,
    NoActiveTurn,
}

impl StopTurnStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StopRequested => "stop_requested",
            Self::AlreadyTerminal => "already_terminal",
            Self::NoActiveTurn => "no_active_turn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalResponseStatus {
    Accepted,
    RejectedStale,
    RejectedAlreadyResolved,
    RejectedTurnNotActive,
}

impl ApprovalResponseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::RejectedStale => "rejected_stale",
            Self::RejectedAlreadyResolved => "rejected_already_resolved",
            Self::RejectedTurnNotActive => "rejected_turn_not_active",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalScope {
    SingleToolCall,
}

impl ApprovalScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SingleToolCall => "single_tool_call",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Reject,
}

impl ApprovalDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Idle,
    Running,
    WaitingApproval,
    Stopping,
}

impl RuntimeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Stopping => "stopping",
        }
    }
}
