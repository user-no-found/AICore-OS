use crate::{
    MemoryAgentWriteRequest, MemoryDecisionId, MemoryFieldVisibility, MemoryProposalId,
    MemoryProposalRecord, MemoryProposalRequest, MemoryProposalReview, MemoryProposalRuntimeError,
    MemoryProposalStatus, MemoryReviewCard, MemorySourceRef, MemoryUserDecision,
    MemoryUserDecisionKind, MemoryWriteBoundaryStatus, MemoryWriteRequestId,
    StoredMemoryProposalReview, validate_proposal_request,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryProposalOutcome {
    pub record: MemoryProposalRecord,
    pub write_request: Option<MemoryAgentWriteRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryProposalValidationOutcome {
    pub accepted: bool,
    pub reason_en: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryProposalStoreSnapshot {
    pub proposals: Vec<MemoryProposalRecord>,
    pub reviews: Vec<StoredMemoryProposalReview>,
    pub decisions: Vec<MemoryUserDecision>,
    pub write_requests: Vec<MemoryAgentWriteRequest>,
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMemoryProposalRuntime {
    proposals: Vec<MemoryProposalRecord>,
    reviews: Vec<StoredMemoryProposalReview>,
    decisions: Vec<MemoryUserDecision>,
    write_requests: Vec<MemoryAgentWriteRequest>,
}

impl InMemoryMemoryProposalRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_proposal(
        &mut self,
        request: MemoryProposalRequest,
    ) -> Result<MemoryProposalOutcome, MemoryProposalRuntimeError> {
        validate_proposal_request(&request)?;
        if self
            .proposals
            .iter()
            .any(|record| record.proposal_id == request.proposal_id)
        {
            return Err(MemoryProposalRuntimeError::DuplicateProposal);
        }
        let record = MemoryProposalRecord::from_request(request);
        self.proposals.push(record.clone());
        Ok(MemoryProposalOutcome {
            record,
            write_request: None,
        })
    }

    pub fn review_proposal(
        &mut self,
        review: MemoryProposalReview,
    ) -> Result<StoredMemoryProposalReview, MemoryProposalRuntimeError> {
        let record = self
            .proposal_mut(&review.proposal_id)
            .ok_or(MemoryProposalRuntimeError::MissingProposal)?;
        if record.target_instance_id != review.target_instance_id {
            return Err(MemoryProposalRuntimeError::ReviewTargetMismatch);
        }
        record.status = MemoryProposalStatus::ReviewReady;
        let stored = StoredMemoryProposalReview::new(review);
        self.reviews.push(stored.clone());
        Ok(stored)
    }

    pub fn build_review_card(
        &self,
        proposal_id: &MemoryProposalId,
    ) -> Result<MemoryReviewCard, MemoryProposalRuntimeError> {
        let record = self
            .proposal(proposal_id)
            .ok_or(MemoryProposalRuntimeError::MissingProposal)?;
        let stored = self
            .latest_review(proposal_id)
            .ok_or(MemoryProposalRuntimeError::MissingReview)?;
        Ok(MemoryReviewCard {
            proposal_id: proposal_id.clone(),
            review_id: stored.review.review_id.clone(),
            proposed_summary_zh: stored.review.review_summary_zh.clone(),
            memory_class: stored.review.target_memory_class,
            reason_en: record.reason_en.clone(),
            target_instance_id: record.target_instance_id.clone(),
            canonical_text_en: stored.review.canonical_text_en.clone(),
            user_annotation_zh: stored.review.user_annotation_zh.clone(),
            user_annotation_zh_visibility: MemoryFieldVisibility::UiOnly,
            user_annotation_enters_model_context: false,
            context_summary_en: record.context_summary_en.clone(),
            risk_flags: stored.review.risk_flags.clone(),
            source_refs: merged_source_refs(record, &stored.review.source_refs),
            available_decisions: vec![
                MemoryUserDecisionKind::ApproveWrite,
                MemoryUserDecisionKind::EditThenWrite,
                MemoryUserDecisionKind::Reject,
                MemoryUserDecisionKind::DeferReview,
            ],
        })
    }

    pub fn record_user_decision(
        &mut self,
        decision: MemoryUserDecision,
    ) -> Result<MemoryProposalOutcome, MemoryProposalRuntimeError> {
        let stored = self
            .latest_review(&decision.proposal_id)
            .cloned()
            .ok_or(MemoryProposalRuntimeError::DecisionRequiresReview)?;
        let idx = self
            .proposal_index(&decision.proposal_id)
            .ok_or(MemoryProposalRuntimeError::MissingProposal)?;

        match decision.decision_kind {
            MemoryUserDecisionKind::ApproveWrite => {
                if !matches!(
                    decision.actor_kind,
                    crate::MemoryProposalSourceKind::MemoryAgent
                ) {
                    return Err(MemoryProposalRuntimeError::WriteRequestRequiresMemoryAgent);
                }
                let write_request = self.create_memory_agent_write_request(&decision, &stored)?;
                self.proposals[idx].status = MemoryProposalStatus::WriteRequested;
                self.proposals[idx].write_boundary_status = MemoryWriteBoundaryStatus::Requested;
                self.decisions.push(decision);
                self.write_requests.push(write_request.clone());
                Ok(MemoryProposalOutcome {
                    record: self.proposals[idx].clone(),
                    write_request: Some(write_request),
                })
            }
            MemoryUserDecisionKind::EditThenWrite => {
                if !matches!(
                    decision.actor_kind,
                    crate::MemoryProposalSourceKind::MemoryAgent
                ) {
                    return Err(MemoryProposalRuntimeError::WriteRequestRequiresMemoryAgent);
                }
                if decision.edited_canonical_text_en.is_none() {
                    return Err(
                        MemoryProposalRuntimeError::EditThenWriteRequiresEditedCanonicalText,
                    );
                }
                let write_request = self.create_memory_agent_write_request(&decision, &stored)?;
                self.proposals[idx].status = MemoryProposalStatus::WriteRequested;
                self.proposals[idx].write_boundary_status = MemoryWriteBoundaryStatus::Requested;
                self.decisions.push(decision);
                self.write_requests.push(write_request.clone());
                Ok(MemoryProposalOutcome {
                    record: self.proposals[idx].clone(),
                    write_request: Some(write_request),
                })
            }
            MemoryUserDecisionKind::Reject => {
                self.proposals[idx].status = MemoryProposalStatus::Rejected;
                self.proposals[idx].write_boundary_status = MemoryWriteBoundaryStatus::Rejected;
                self.decisions.push(decision);
                Ok(MemoryProposalOutcome {
                    record: self.proposals[idx].clone(),
                    write_request: None,
                })
            }
            MemoryUserDecisionKind::DeferReview => {
                self.proposals[idx].status = MemoryProposalStatus::Deferred;
                self.decisions.push(decision);
                Ok(MemoryProposalOutcome {
                    record: self.proposals[idx].clone(),
                    write_request: None,
                })
            }
        }
    }

    pub fn create_memory_agent_write_request(
        &self,
        decision: &MemoryUserDecision,
        stored: &StoredMemoryProposalReview,
    ) -> Result<MemoryAgentWriteRequest, MemoryProposalRuntimeError> {
        if !matches!(
            decision.actor_kind,
            crate::MemoryProposalSourceKind::MemoryAgent
        ) {
            return Err(MemoryProposalRuntimeError::WriteRequestRequiresMemoryAgent);
        }
        let canonical_text_en = decision
            .edited_canonical_text_en
            .clone()
            .unwrap_or_else(|| stored.review.canonical_text_en.clone());
        let user_annotation_zh = decision
            .edited_user_annotation_zh
            .clone()
            .unwrap_or_else(|| stored.review.user_annotation_zh.clone());
        Ok(MemoryAgentWriteRequest {
            write_request_id: write_request_id_from_decision(&decision.decision_id),
            proposal_id: decision.proposal_id.clone(),
            decision_id: decision.decision_id.clone(),
            target_instance_id: stored.review.target_instance_id.clone(),
            target_memory_class: stored.review.target_memory_class,
            canonical_text_en,
            user_annotation_zh,
            status: MemoryWriteBoundaryStatus::Requested,
            applied: false,
            created_at: decision.decided_at,
        })
    }

    pub fn get_proposal(&self, proposal_id: &MemoryProposalId) -> Option<&MemoryProposalRecord> {
        self.proposal(proposal_id)
    }

    pub fn list_pending_reviews(&self) -> Vec<&MemoryProposalRecord> {
        self.proposals
            .iter()
            .filter(|record| {
                matches!(
                    record.status,
                    MemoryProposalStatus::PendingReview | MemoryProposalStatus::Deferred
                )
            })
            .collect()
    }

    pub fn proposal_can_enter_memory_context(&self, proposal_id: &MemoryProposalId) -> bool {
        self.proposal(proposal_id)
            .map(|record| record.status == MemoryProposalStatus::Written && record.write_applied)
            .unwrap_or(false)
    }

    pub fn snapshot(&self) -> MemoryProposalStoreSnapshot {
        MemoryProposalStoreSnapshot {
            proposals: self.proposals.clone(),
            reviews: self.reviews.clone(),
            decisions: self.decisions.clone(),
            write_requests: self.write_requests.clone(),
        }
    }

    fn proposal(&self, proposal_id: &MemoryProposalId) -> Option<&MemoryProposalRecord> {
        self.proposals
            .iter()
            .find(|record| &record.proposal_id == proposal_id)
    }

    fn proposal_mut(
        &mut self,
        proposal_id: &MemoryProposalId,
    ) -> Option<&mut MemoryProposalRecord> {
        self.proposals
            .iter_mut()
            .find(|record| &record.proposal_id == proposal_id)
    }

    fn proposal_index(&self, proposal_id: &MemoryProposalId) -> Option<usize> {
        self.proposals
            .iter()
            .position(|record| &record.proposal_id == proposal_id)
    }

    fn latest_review(&self, proposal_id: &MemoryProposalId) -> Option<&StoredMemoryProposalReview> {
        self.reviews
            .iter()
            .rev()
            .find(|review| &review.review.proposal_id == proposal_id)
    }
}

fn write_request_id_from_decision(decision_id: &MemoryDecisionId) -> MemoryWriteRequestId {
    MemoryWriteRequestId::new(format!("write.{}", decision_id.as_str()))
        .expect("derived id is valid")
}

fn merged_source_refs(
    record: &MemoryProposalRecord,
    review_refs: &[MemorySourceRef],
) -> Vec<MemorySourceRef> {
    if review_refs.is_empty() {
        record.source_refs.clone()
    } else {
        review_refs.to_vec()
    }
}
