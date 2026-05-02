use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

use super::{MemoryDecisionId, MemoryProposalId, MemoryProposalSourceKind, MemoryUserDecisionKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryUserDecision {
    pub decision_id: MemoryDecisionId,
    pub proposal_id: MemoryProposalId,
    pub actor_kind: MemoryProposalSourceKind,
    pub decision_kind: MemoryUserDecisionKind,
    pub edited_canonical_text_en: Option<String>,
    pub edited_user_annotation_zh: Option<String>,
    pub decided_at: Timestamp,
}
