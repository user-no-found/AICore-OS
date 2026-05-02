use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use serde::{Deserialize, Serialize};

use super::{
    MemoryClass, MemoryProposalId, MemoryProposalSourceKind, MemoryProposalStatus,
    MemoryVisibilityScope, MemoryWriteBoundaryStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySourceRef {
    pub source_instance_id: InstanceId,
    pub source_workspace_path: Option<String>,
    pub source_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalRequest {
    pub proposal_id: MemoryProposalId,
    pub target_instance_id: InstanceId,
    pub source_instance_id: InstanceId,
    pub source_session_id: Option<SessionId>,
    pub source_turn_id: Option<TurnId>,
    pub source_actor_kind: MemoryProposalSourceKind,
    pub source_actor_id: Option<String>,
    pub source_refs: Vec<MemorySourceRef>,
    pub proposed_memory_class: MemoryClass,
    pub reason_en: String,
    pub context_summary_en: String,
    pub candidate_text_en: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalRecord {
    pub proposal_id: MemoryProposalId,
    pub target_instance_id: InstanceId,
    pub source_instance_id: InstanceId,
    pub source_session_id: Option<SessionId>,
    pub source_turn_id: Option<TurnId>,
    pub source_actor_kind: MemoryProposalSourceKind,
    pub source_actor_id: Option<String>,
    pub source_refs: Vec<MemorySourceRef>,
    pub proposed_memory_class: MemoryClass,
    pub reason_en: String,
    pub context_summary_en: String,
    pub candidate_text_en: String,
    pub visibility_scope: MemoryVisibilityScope,
    pub status: MemoryProposalStatus,
    pub write_boundary_status: MemoryWriteBoundaryStatus,
    pub write_applied: bool,
    pub created_at: Timestamp,
}

impl MemoryProposalRecord {
    pub fn from_request(request: MemoryProposalRequest) -> Self {
        Self {
            proposal_id: request.proposal_id,
            target_instance_id: request.target_instance_id,
            source_instance_id: request.source_instance_id,
            source_session_id: request.source_session_id,
            source_turn_id: request.source_turn_id,
            source_actor_kind: request.source_actor_kind,
            source_actor_id: request.source_actor_id,
            source_refs: request.source_refs,
            proposed_memory_class: request.proposed_memory_class,
            reason_en: request.reason_en,
            context_summary_en: request.context_summary_en,
            candidate_text_en: request.candidate_text_en,
            visibility_scope: MemoryVisibilityScope::CurrentInstanceOnly,
            status: MemoryProposalStatus::PendingReview,
            write_boundary_status: MemoryWriteBoundaryStatus::NotRequested,
            write_applied: false,
            created_at: request.created_at,
        }
    }
}
