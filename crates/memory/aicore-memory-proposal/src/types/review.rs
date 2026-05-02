use aicore_foundation::{InstanceId, Timestamp};
use serde::{Deserialize, Serialize};

use super::{
    MemoryClass, MemoryFieldVisibility, MemoryProposalId, MemoryProposalStatus, MemoryReviewId,
    MemoryRiskFlag, MemorySourceRef, MemoryUserDecisionKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProposalReview {
    pub review_id: MemoryReviewId,
    pub proposal_id: MemoryProposalId,
    pub target_instance_id: InstanceId,
    pub target_memory_class: MemoryClass,
    pub dedupe_summary_en: String,
    pub risk_flags: Vec<MemoryRiskFlag>,
    pub canonical_text_en: String,
    pub user_annotation_zh: String,
    pub review_summary_zh: String,
    pub source_refs: Vec<MemorySourceRef>,
    pub recommended_decision: MemoryUserDecisionKind,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMemoryProposalReview {
    pub review: MemoryProposalReview,
    pub status_after_review: MemoryProposalStatus,
    pub canonical_text_visibility: MemoryFieldVisibility,
    pub user_annotation_zh_visibility: MemoryFieldVisibility,
    pub user_annotation_enters_model_context: bool,
}

impl StoredMemoryProposalReview {
    pub fn new(review: MemoryProposalReview) -> Self {
        Self {
            review,
            status_after_review: MemoryProposalStatus::ReviewReady,
            canonical_text_visibility: MemoryFieldVisibility::ModelFacing,
            user_annotation_zh_visibility: MemoryFieldVisibility::UiOnly,
            user_annotation_enters_model_context: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryReviewCard {
    pub proposal_id: MemoryProposalId,
    pub review_id: MemoryReviewId,
    pub proposed_summary_zh: String,
    pub memory_class: MemoryClass,
    pub reason_en: String,
    pub target_instance_id: InstanceId,
    pub canonical_text_en: String,
    pub user_annotation_zh: String,
    pub user_annotation_zh_visibility: MemoryFieldVisibility,
    pub user_annotation_enters_model_context: bool,
    pub context_summary_en: String,
    pub risk_flags: Vec<MemoryRiskFlag>,
    pub source_refs: Vec<MemorySourceRef>,
    pub available_decisions: Vec<MemoryUserDecisionKind>,
}
