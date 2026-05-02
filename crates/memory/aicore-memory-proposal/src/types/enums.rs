use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProposalStatus {
    Draft,
    PendingReview,
    ReviewReady,
    Approved,
    Edited,
    Rejected,
    Deferred,
    WriteRequested,
    Written,
    Failed,
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClass {
    Permanent,
    Long,
    StrongPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProposalSourceKind {
    Agent,
    TeamAgent,
    User,
    Gateway,
    MemoryAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRiskFlag {
    ContainsSecretRisk,
    RawLogRisk,
    UnverifiedClaim,
    CrossInstanceRisk,
    DuplicateCandidate,
    LowReuseValue,
    Safe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryUserDecisionKind {
    ApproveWrite,
    EditThenWrite,
    Reject,
    DeferReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryWriteBoundaryStatus {
    NotRequested,
    Requested,
    AppliedByMemoryAgent,
    Rejected,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryVisibilityScope {
    CurrentInstanceOnly,
    GlobalMainInspectResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryFieldVisibility {
    ModelFacing,
    UiOnly,
}
