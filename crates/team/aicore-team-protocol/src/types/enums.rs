use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRunStatus {
    Created,
    Running,
    Stopping,
    Stopped,
    Completed,
    Failed,
    Destroyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamAgentStatus {
    Created,
    Running,
    WaitingTool,
    WaitingApproval,
    Completed,
    Failed,
    Stopping,
    Stopped,
    Expired,
    Destroyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamChannelStatus {
    Open,
    Closed,
    Destroyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamMessageKind {
    AgentMessage,
    Finding,
    Question,
    Answer,
    Status,
    ResultSummary,
    StopNotice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamCommunicationScope {
    MainVisible,
    TeamVisible,
    Directed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamResultStatus {
    Submitted,
    Accepted,
    LateIgnored,
    RejectedChannelClosed,
    RejectedTurnStopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSpawnFailureCode {
    TurnNotActive,
    ChannelClosed,
    TooManyAgents,
    ConcurrencyLimit,
    SpawnDepthExceeded,
    InvalidModel,
    ToolNotAllowed,
    BudgetMissing,
}
