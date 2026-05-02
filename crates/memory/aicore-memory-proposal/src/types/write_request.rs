use aicore_foundation::{InstanceId, Timestamp};
use serde::{Deserialize, Serialize};

use super::{
    MemoryClass, MemoryDecisionId, MemoryProposalId, MemoryWriteBoundaryStatus,
    MemoryWriteRequestId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAgentWriteRequest {
    pub write_request_id: MemoryWriteRequestId,
    pub proposal_id: MemoryProposalId,
    pub decision_id: MemoryDecisionId,
    pub target_instance_id: InstanceId,
    pub target_memory_class: MemoryClass,
    pub canonical_text_en: String,
    pub user_annotation_zh: String,
    pub status: MemoryWriteBoundaryStatus,
    pub applied: bool,
    pub created_at: Timestamp,
}
