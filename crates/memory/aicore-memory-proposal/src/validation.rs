use aicore_foundation::InstanceId;

use crate::{MemoryProposalRequest, MemoryProposalSourceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryProposalRuntimeError {
    DuplicateProposal,
    MissingProposal,
    MissingReview,
    CrossInstanceProposalRejected,
    ReviewTargetMismatch,
    DecisionRequiresReview,
    EditThenWriteRequiresEditedCanonicalText,
    WriteRequestRequiresMemoryAgent,
}

pub fn validate_proposal_request(
    request: &MemoryProposalRequest,
) -> Result<(), MemoryProposalRuntimeError> {
    if request.target_instance_id != request.source_instance_id {
        return Err(MemoryProposalRuntimeError::CrossInstanceProposalRejected);
    }
    if request.target_instance_id == InstanceId::global_main()
        && !matches!(
            request.source_actor_kind,
            MemoryProposalSourceKind::MemoryAgent
        )
    {
        return Err(MemoryProposalRuntimeError::CrossInstanceProposalRejected);
    }
    if request
        .source_refs
        .iter()
        .any(|source_ref| source_ref.source_instance_id != request.source_instance_id)
    {
        return Err(MemoryProposalRuntimeError::CrossInstanceProposalRejected);
    }
    Ok(())
}
