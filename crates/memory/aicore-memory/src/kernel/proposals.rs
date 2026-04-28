use super::*;

impl MemoryKernel {
    pub fn submit_assistant_summary(
        &mut self,
        scope: MemoryScope,
        content: &str,
    ) -> Result<MemoryProposalId, MemoryError> {
        let _guard = self.acquire_write_guard("submit_assistant_summary")?;
        self.maybe_fail_write_for_tests()?;
        let proposal = MemoryProposal {
            proposal_id: next_id("prop"),
            memory_type: MemoryType::Working,
            scope: scope.clone(),
            source: MemorySource::AssistantSummary,
            status: MemoryProposalStatus::Open,
            content: content.to_string(),
            content_language: infer_language(content).to_string(),
            normalized_content: content.to_string(),
            normalized_language: infer_language(content).to_string(),
            localized_summary: content.to_string(),
            created_at: now_string(),
        };
        let event = MemoryEvent {
            event_id: next_id("evt"),
            event_kind: MemoryEventKind::Proposed,
            memory_id: None,
            proposal_id: Some(proposal.proposal_id.clone()),
            scope,
            actor: "assistant".to_string(),
            reason: Some("assistant summary".to_string()),
            evidence_json: "[]".to_string(),
            created_at: proposal.created_at.clone(),
        };

        block_on(async {
            store::insert_proposal_and_event(&self.paths.db_path, &proposal, &event).await
        })?;
        self.refresh_cache()?;
        Ok(proposal.proposal_id.clone())
    }

    pub fn submit_agent_output(
        &mut self,
        output: MemoryAgentOutput,
    ) -> Result<Vec<MemoryProposalId>, MemoryError> {
        if output.proposals.is_empty() {
            return Ok(Vec::new());
        }

        let _guard = self.acquire_write_guard("submit_agent_output")?;
        self.maybe_fail_write_for_tests()?;

        let mut existing_open = self
            .proposals
            .iter()
            .filter(|proposal| proposal.status == MemoryProposalStatus::Open)
            .cloned()
            .collect::<Vec<_>>();
        let mut inserted_ids = Vec::new();

        for incoming in output.proposals {
            let already_exists = existing_open.iter().any(|proposal| {
                proposal.scope == incoming.scope
                    && proposal.memory_type == incoming.memory_type
                    && proposal.normalized_content == incoming.normalized_content
            });
            if already_exists {
                continue;
            }

            let timestamp = now_string();
            let proposal = MemoryProposal {
                proposal_id: next_id("prop"),
                memory_type: incoming.memory_type,
                scope: incoming.scope,
                source: incoming.source,
                status: MemoryProposalStatus::Open,
                content: incoming.content,
                content_language: incoming.content_language,
                normalized_content: incoming.normalized_content,
                normalized_language: incoming.normalized_language,
                localized_summary: incoming.localized_summary,
                created_at: timestamp.clone(),
            };
            let event = MemoryEvent {
                event_id: next_id("evt"),
                event_kind: MemoryEventKind::Proposed,
                memory_id: None,
                proposal_id: Some(proposal.proposal_id.clone()),
                scope: proposal.scope.clone(),
                actor: "memory-agent".to_string(),
                reason: Some("agent proposal".to_string()),
                evidence_json: "[]".to_string(),
                created_at: timestamp,
            };

            block_on(async {
                store::insert_proposal_and_event(&self.paths.db_path, &proposal, &event).await
            })?;
            inserted_ids.push(proposal.proposal_id.clone());
            existing_open.push(proposal);
        }

        self.refresh_cache()?;
        Ok(inserted_ids)
    }

    pub fn list_open_proposals(&self) -> Vec<MemoryProposal> {
        self.proposals
            .iter()
            .filter(|proposal| proposal.status == MemoryProposalStatus::Open)
            .cloned()
            .collect()
    }

    pub fn accept_proposal(
        &mut self,
        proposal_id: &str,
        actor: &str,
        reason: Option<&str>,
    ) -> Result<MemoryId, MemoryError> {
        let _guard = self.acquire_write_guard("accept_proposal")?;
        self.maybe_fail_write_for_tests()?;

        let proposal = self
            .proposals
            .iter()
            .find(|proposal| proposal.proposal_id == proposal_id)
            .cloned()
            .ok_or_else(|| MemoryError(format!("unknown proposal_id: {proposal_id}")))?;

        if proposal.status != MemoryProposalStatus::Open {
            return Err(MemoryError(format!("non-open proposal: {proposal_id}")));
        }

        let timestamp = now_string();
        let memory_id = next_id("mem");
        let record = MemoryRecord {
            memory_id: memory_id.clone(),
            record_version: 1,
            memory_type: proposal.memory_type,
            status: MemoryStatus::Active,
            permanence: MemoryPermanence::Standard,
            scope: proposal.scope.clone(),
            content: proposal.content.clone(),
            content_language: proposal.content_language.clone(),
            normalized_content: proposal.normalized_content.clone(),
            normalized_language: proposal.normalized_language.clone(),
            localized_summary: proposal.localized_summary.clone(),
            source: proposal.source.clone(),
            evidence_json: "[]".to_string(),
            state_key: None,
            state_version: 1,
            current_state: None,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };
        let event = MemoryEvent {
            event_id: next_id("evt"),
            event_kind: MemoryEventKind::Accepted,
            memory_id: Some(memory_id.clone()),
            proposal_id: Some(proposal_id.to_string()),
            scope: proposal.scope,
            actor: actor.to_string(),
            reason: reason.map(ToString::to_string),
            evidence_json: "[]".to_string(),
            created_at: timestamp,
        };

        block_on(async {
            store::accept_proposal(&self.paths.db_path, proposal_id, &record, &event).await
        })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()?;
        Ok(memory_id)
    }

    pub fn reject_proposal(
        &mut self,
        proposal_id: &str,
        actor: &str,
        reason: Option<&str>,
    ) -> Result<(), MemoryError> {
        let _guard = self.acquire_write_guard("reject_proposal")?;
        self.maybe_fail_write_for_tests()?;

        let proposal = self
            .proposals
            .iter()
            .find(|proposal| proposal.proposal_id == proposal_id)
            .cloned()
            .ok_or_else(|| MemoryError(format!("unknown proposal_id: {proposal_id}")))?;

        if proposal.status != MemoryProposalStatus::Open {
            return Err(MemoryError(format!("non-open proposal: {proposal_id}")));
        }

        let event = MemoryEvent {
            event_id: next_id("evt"),
            event_kind: MemoryEventKind::Rejected,
            memory_id: None,
            proposal_id: Some(proposal_id.to_string()),
            scope: proposal.scope,
            actor: actor.to_string(),
            reason: reason.map(ToString::to_string),
            evidence_json: "[]".to_string(),
            created_at: now_string(),
        };

        block_on(async { store::reject_proposal(&self.paths.db_path, proposal_id, &event).await })?;
        self.refresh_cache()?;
        Ok(())
    }
}
