use crate::ids::{MemoryEventId, MemoryId, MemoryProposalId, MemorySnapshotRev};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryScope {
    GlobalMain {
        instance_id: String,
    },
    Workspace {
        instance_id: String,
        workspace_root: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryType {
    Core,
    Working,
    Status,
    Decision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryStatus {
    Active,
    Superseded,
    Invalidated,
    Archived,
    Forgotten,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryPermanence {
    Standard,
    Permanent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemorySource {
    UserExplicit,
    UserCorrection,
    AssistantSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryProposalStatus {
    Open,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryEventKind {
    Accepted,
    Proposed,
    Corrected,
    Archived,
    Forgotten,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub memory_id: MemoryId,
    pub record_version: i64,
    pub memory_type: MemoryType,
    pub status: MemoryStatus,
    pub permanence: MemoryPermanence,
    pub scope: MemoryScope,
    pub content: String,
    pub content_language: String,
    pub normalized_content: String,
    pub normalized_language: String,
    pub localized_summary: String,
    pub source: MemorySource,
    pub evidence_json: String,
    pub state_key: Option<String>,
    pub state_version: i64,
    pub current_state: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryProposal {
    pub proposal_id: MemoryProposalId,
    pub memory_type: MemoryType,
    pub scope: MemoryScope,
    pub source: MemorySource,
    pub status: MemoryProposalStatus,
    pub content: String,
    pub content_language: String,
    pub normalized_content: String,
    pub normalized_language: String,
    pub localized_summary: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEdge {
    pub from_memory_id: MemoryId,
    pub to_memory_id: MemoryId,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEvent {
    pub event_id: MemoryEventId,
    pub event_kind: MemoryEventKind,
    pub memory_id: Option<MemoryId>,
    pub proposal_id: Option<MemoryProposalId>,
    pub scope: MemoryScope,
    pub actor: String,
    pub reason: Option<String>,
    pub evidence_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySnapshot {
    pub rev: MemorySnapshotRev,
    pub core_markdown: String,
    pub status_markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionState {
    pub stale: bool,
    pub warning: Option<String>,
    pub last_rebuild_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RememberInput {
    pub memory_type: MemoryType,
    pub permanence: MemoryPermanence,
    pub scope: MemoryScope,
    pub content: String,
    pub localized_summary: String,
    pub state_key: Option<String>,
    pub current_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub scope: Option<MemoryScope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryError(pub String);
