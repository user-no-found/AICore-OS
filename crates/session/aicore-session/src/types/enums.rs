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
    Completed,
    Interrupted,
    Cancelled,
}

impl TurnStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
            Self::Cancelled => "cancelled",
        }
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
pub enum ControlEventType {
    SessionCreated,
    TurnBegan,
    TurnFinished,
    MessageAppended,
    TurnInterrupted,
    RuntimeStateUpdated,
}

impl ControlEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SessionCreated => "session_created",
            Self::TurnBegan => "turn_began",
            Self::TurnFinished => "turn_finished",
            Self::MessageAppended => "message_appended",
            Self::TurnInterrupted => "turn_interrupted",
            Self::RuntimeStateUpdated => "runtime_state_updated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerWriteType {
    Insert,
    Update,
    Delete,
}

impl LedgerWriteType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Update => "update",
            Self::Delete => "delete",
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
