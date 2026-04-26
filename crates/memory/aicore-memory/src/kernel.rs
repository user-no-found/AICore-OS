use std::{
    fs,
    future::Future,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::runtime::Builder;

use crate::{
    ids::{MemoryId, MemoryProposalId},
    lock::MemoryWriteGuard,
    paths::MemoryPaths,
    projection::{
        build_core_projection, build_decisions_projection, build_permanent_projection,
        build_status_projection, rebuild_projections,
    },
    search::{build_memory_pack, filter_records, filter_records_by_ids},
    store,
    types::{
        MemoryAgentOutput, MemoryAuditReport, MemoryEdge, MemoryError, MemoryEvent,
        MemoryEventKind, MemoryPermanence, MemoryProposal, MemoryProposalStatus, MemoryRecord,
        MemoryScope, MemorySource, MemoryStatus, MemoryType, ProjectionState, RememberInput,
        SearchQuery, SearchResult,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryKernel {
    paths: MemoryPaths,
    records: Vec<MemoryRecord>,
    proposals: Vec<MemoryProposal>,
    events: Vec<MemoryEvent>,
    edges: Vec<MemoryEdge>,
    projection_state: ProjectionState,
    projection_should_fail_for_tests: bool,
    write_should_fail_for_tests: bool,
}

impl MemoryKernel {
    pub fn open(paths: MemoryPaths) -> Result<Self, MemoryError> {
        fs::create_dir_all(&paths.root).map_err(|error| MemoryError(error.to_string()))?;
        fs::create_dir_all(&paths.projections_dir)
            .map_err(|error| MemoryError(error.to_string()))?;

        block_on(async { store::init_schema(&paths.db_path).await })?;

        let mut kernel = Self {
            paths,
            records: Vec::new(),
            proposals: Vec::new(),
            events: Vec::new(),
            edges: Vec::new(),
            projection_state: ProjectionState {
                stale: false,
                warning: None,
                last_rebuild_at: None,
            },
            projection_should_fail_for_tests: false,
            write_should_fail_for_tests: false,
        };

        kernel.refresh_cache()?;
        Ok(kernel)
    }

    pub fn table_names(&self) -> Result<Vec<String>, MemoryError> {
        block_on(async { store::table_names(&self.paths.db_path).await })
    }

    pub fn remember_user_explicit(
        &mut self,
        input: RememberInput,
    ) -> Result<MemoryId, MemoryError> {
        let _guard = self.acquire_write_guard("remember_user_explicit")?;
        self.maybe_fail_write_for_tests()?;
        let timestamp = now_string();
        let memory_id = next_id("mem");
        let event_id = next_id("evt");
        let content_language = infer_language(&input.content).to_string();
        let normalized = input.content.clone();

        let record = MemoryRecord {
            memory_id: memory_id.clone(),
            record_version: 1,
            memory_type: input.memory_type,
            status: MemoryStatus::Active,
            permanence: input.permanence,
            scope: input.scope.clone(),
            content: input.content,
            content_language: content_language.clone(),
            normalized_content: normalized,
            normalized_language: content_language,
            localized_summary: input.localized_summary,
            source: MemorySource::UserExplicit,
            evidence_json: "[]".to_string(),
            state_key: input.state_key,
            state_version: 1,
            current_state: input.current_state,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };

        let event = MemoryEvent {
            event_id,
            event_kind: MemoryEventKind::Accepted,
            memory_id: Some(memory_id.clone()),
            proposal_id: None,
            scope: input.scope,
            actor: "user".to_string(),
            reason: Some("remember".to_string()),
            evidence_json: "[]".to_string(),
            created_at: timestamp,
        };

        block_on(async {
            store::insert_record_and_event(&self.paths.db_path, &record, &event).await
        })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()?;

        Ok(memory_id)
    }

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

    pub fn correct_by_user(
        &mut self,
        old_memory_id: &str,
        new_content: &str,
    ) -> Result<MemoryId, MemoryError> {
        let expected_version = self
            .records
            .iter()
            .find(|record| record.memory_id == old_memory_id)
            .map(|record| record.record_version)
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {old_memory_id}")))?;

        self.correct_by_user_with_version(old_memory_id, expected_version, new_content)
    }

    pub fn correct_by_user_with_version(
        &mut self,
        old_memory_id: &str,
        expected_version: i64,
        new_content: &str,
    ) -> Result<MemoryId, MemoryError> {
        let _guard = self.acquire_write_guard("correct_by_user_with_version")?;
        self.maybe_fail_write_for_tests()?;
        let old_record = self
            .records
            .iter()
            .find(|record| record.memory_id == old_memory_id)
            .cloned()
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {old_memory_id}")))?;

        let timestamp = now_string();
        let new_memory_id = next_id("mem");
        let content_language = infer_language(new_content).to_string();
        let record = MemoryRecord {
            memory_id: new_memory_id.clone(),
            record_version: 1,
            memory_type: old_record.memory_type,
            status: MemoryStatus::Active,
            permanence: old_record.permanence,
            scope: old_record.scope.clone(),
            content: new_content.to_string(),
            content_language: content_language.clone(),
            normalized_content: new_content.to_string(),
            normalized_language: content_language,
            localized_summary: new_content.to_string(),
            source: MemorySource::UserCorrection,
            evidence_json: "[]".to_string(),
            state_key: old_record.state_key,
            state_version: old_record.state_version + 1,
            current_state: old_record.current_state,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };
        let event = MemoryEvent {
            event_id: next_id("evt"),
            event_kind: MemoryEventKind::Corrected,
            memory_id: Some(new_memory_id.clone()),
            proposal_id: None,
            scope: old_record.scope,
            actor: "user".to_string(),
            reason: Some(format!("supersedes {old_memory_id}")),
            evidence_json: "[]".to_string(),
            created_at: timestamp,
        };

        block_on(async {
            store::supersede_record(
                &self.paths.db_path,
                old_memory_id,
                expected_version,
                &record,
                &event,
            )
            .await
        })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()?;

        Ok(new_memory_id)
    }

    pub fn archive(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        let expected_version = self
            .records
            .iter()
            .find(|record| record.memory_id == memory_id)
            .map(|record| record.record_version)
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {memory_id}")))?;

        self.archive_with_version(memory_id, expected_version)
    }

    pub fn archive_with_version(
        &mut self,
        memory_id: &str,
        expected_version: i64,
    ) -> Result<(), MemoryError> {
        let _guard = self.acquire_write_guard("archive_with_version")?;
        self.maybe_fail_write_for_tests()?;
        self.update_status(
            memory_id,
            expected_version,
            MemoryStatus::Archived,
            MemoryEventKind::Archived,
        )
    }

    pub fn forget(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        let expected_version = self
            .records
            .iter()
            .find(|record| record.memory_id == memory_id)
            .map(|record| record.record_version)
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {memory_id}")))?;

        self.forget_with_version(memory_id, expected_version)
    }

    pub fn forget_with_version(
        &mut self,
        memory_id: &str,
        expected_version: i64,
    ) -> Result<(), MemoryError> {
        let _guard = self.acquire_write_guard("forget_with_version")?;
        self.maybe_fail_write_for_tests()?;
        self.update_status(
            memory_id,
            expected_version,
            MemoryStatus::Forgotten,
            MemoryEventKind::Forgotten,
        )
    }

    pub fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, MemoryError> {
        let candidate_ids = block_on(async {
            store::search_index_candidates(&self.paths.db_path, &query.text, query.limit).await
        })?;

        Ok(match candidate_ids {
            Some(candidate_ids) if !query.text.is_empty() && !candidate_ids.is_empty() => {
                filter_records_by_ids(&self.records, &query, &candidate_ids)
            }
            _ => filter_records(&self.records, &query),
        })
    }

    pub fn build_memory_context_pack(
        &self,
        query: SearchQuery,
        token_budget: usize,
    ) -> Vec<MemoryRecord> {
        build_memory_pack(&self.records, &query, token_budget)
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

    pub fn records(&self) -> &[MemoryRecord] {
        &self.records
    }

    pub fn proposals(&self) -> &[MemoryProposal] {
        &self.proposals
    }

    pub fn events(&self) -> &[MemoryEvent] {
        &self.events
    }

    pub fn edges(&self) -> &[MemoryEdge] {
        &self.edges
    }

    pub fn projection_state(&self) -> &ProjectionState {
        &self.projection_state
    }

    pub fn verify_ledger_consistency(&self) -> MemoryAuditReport {
        let mut issues = Vec::new();

        for event in &self.events {
            match event.event_kind {
                MemoryEventKind::Accepted => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "accepted event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    if !self
                        .records
                        .iter()
                        .any(|record| record.memory_id == memory_id)
                    {
                        issues.push(format!(
                            "accepted event {} points to missing memory {}",
                            event.event_id, memory_id
                        ));
                    }

                    if let Some(proposal_id) = event.proposal_id.as_deref() {
                        match self
                            .proposals
                            .iter()
                            .find(|proposal| proposal.proposal_id == proposal_id)
                        {
                            Some(proposal) if proposal.status == MemoryProposalStatus::Accepted => {
                            }
                            Some(_) => issues.push(format!(
                                "accepted event {} points to non-accepted proposal {}",
                                event.event_id, proposal_id
                            )),
                            None => issues.push(format!(
                                "accepted event {} points to missing proposal {}",
                                event.event_id, proposal_id
                            )),
                        }
                    }
                }
                MemoryEventKind::Proposed => {
                    let Some(proposal_id) = event.proposal_id.as_deref() else {
                        issues.push(format!(
                            "proposed event {} missing proposal_id",
                            event.event_id
                        ));
                        continue;
                    };

                    if !self
                        .proposals
                        .iter()
                        .any(|proposal| proposal.proposal_id == proposal_id)
                    {
                        issues.push(format!(
                            "proposed event {} points to missing proposal {}",
                            event.event_id, proposal_id
                        ));
                    }
                }
                MemoryEventKind::Rejected => {
                    let Some(proposal_id) = event.proposal_id.as_deref() else {
                        issues.push(format!(
                            "rejected event {} missing proposal_id",
                            event.event_id
                        ));
                        continue;
                    };

                    match self
                        .proposals
                        .iter()
                        .find(|proposal| proposal.proposal_id == proposal_id)
                    {
                        Some(proposal) if proposal.status == MemoryProposalStatus::Rejected => {}
                        Some(_) => issues.push(format!(
                            "rejected event {} points to non-rejected proposal {}",
                            event.event_id, proposal_id
                        )),
                        None => issues.push(format!(
                            "rejected event {} points to missing proposal {}",
                            event.event_id, proposal_id
                        )),
                    }
                }
                MemoryEventKind::Corrected => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "corrected event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    let Some(record) = self
                        .records
                        .iter()
                        .find(|record| record.memory_id == memory_id)
                    else {
                        issues.push(format!(
                            "corrected event {} points to missing memory {}",
                            event.event_id, memory_id
                        ));
                        continue;
                    };

                    if record.status != MemoryStatus::Active {
                        issues.push(format!(
                            "corrected event {} points to non-active memory {}",
                            event.event_id, memory_id
                        ));
                    }

                    let old_memory_id = event
                        .reason
                        .as_deref()
                        .and_then(|reason| reason.strip_prefix("supersedes "));
                    match old_memory_id {
                        Some(old_memory_id) => {
                            if !self.edges.iter().any(|edge| {
                                edge.from_memory_id == memory_id
                                    && edge.to_memory_id == old_memory_id
                                    && edge.relation == "supersedes"
                            }) {
                                issues.push(format!(
                                    "corrected event {} missing supersedes edge {} -> {}",
                                    event.event_id, memory_id, old_memory_id
                                ));
                            }
                        }
                        None => issues.push(format!(
                            "corrected event {} missing supersedes reason",
                            event.event_id
                        )),
                    }
                }
                MemoryEventKind::Archived => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "archived event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    match self
                        .records
                        .iter()
                        .find(|record| record.memory_id == memory_id)
                    {
                        Some(record) if record.status == MemoryStatus::Archived => {}
                        Some(_) => issues.push(format!(
                            "archived event {} points to non-archived memory {}",
                            event.event_id, memory_id
                        )),
                        None => issues.push(format!(
                            "archived event {} points to missing memory {}",
                            event.event_id, memory_id
                        )),
                    }
                }
                MemoryEventKind::Forgotten => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "forgotten event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    match self
                        .records
                        .iter()
                        .find(|record| record.memory_id == memory_id)
                    {
                        Some(record) if record.status == MemoryStatus::Forgotten => {}
                        Some(_) => issues.push(format!(
                            "forgotten event {} points to non-forgotten memory {}",
                            event.event_id, memory_id
                        )),
                        None => issues.push(format!(
                            "forgotten event {} points to missing memory {}",
                            event.event_id, memory_id
                        )),
                    }
                }
            }
        }

        for proposal in &self.proposals {
            if !self.events.iter().any(|event| {
                event.event_kind == MemoryEventKind::Proposed
                    && event.proposal_id.as_deref() == Some(proposal.proposal_id.as_str())
            }) {
                issues.push(format!(
                    "proposal {} missing proposed event",
                    proposal.proposal_id
                ));
            }
        }

        MemoryAuditReport {
            ok: issues.is_empty(),
            checked_events: self.events.len(),
            issues,
        }
    }

    pub fn core_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.core_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn status_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.status_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_index_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_index_md)
            .map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_core_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_core_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_decisions_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_decisions_md)
            .map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_status_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_status_md)
            .map_err(|error| MemoryError(error.to_string()))
    }

    pub fn set_projection_failure_for_tests(&mut self, should_fail: bool) {
        self.projection_should_fail_for_tests = should_fail;
    }

    #[cfg(test)]
    pub fn set_write_failure_for_tests(&mut self, should_fail: bool) {
        self.write_should_fail_for_tests = should_fail;
    }

    #[cfg(test)]
    pub fn delete_record_for_tests(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        block_on(async { store::delete_record_for_tests(&self.paths.db_path, memory_id).await })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn delete_proposal_for_tests(&mut self, proposal_id: &str) -> Result<(), MemoryError> {
        block_on(async {
            store::delete_proposal_for_tests(&self.paths.db_path, proposal_id).await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn delete_edge_for_tests(
        &mut self,
        from_memory_id: &str,
        to_memory_id: &str,
        relation: &str,
    ) -> Result<(), MemoryError> {
        block_on(async {
            store::delete_edge_for_tests(
                &self.paths.db_path,
                from_memory_id,
                to_memory_id,
                relation,
            )
            .await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn force_record_status_for_tests(
        &mut self,
        memory_id: &str,
        status: MemoryStatus,
    ) -> Result<(), MemoryError> {
        block_on(async {
            store::force_record_status_for_tests(&self.paths.db_path, memory_id, status).await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn force_normalized_content_for_tests(
        &mut self,
        memory_id: &str,
        normalized_content: &str,
    ) -> Result<(), MemoryError> {
        block_on(async {
            store::force_normalized_content_for_tests(
                &self.paths.db_path,
                memory_id,
                normalized_content,
            )
            .await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn search_index_available_for_tests(&self) -> Result<bool, MemoryError> {
        block_on(async { store::search_index_available(&self.paths.db_path).await })
    }

    #[cfg(test)]
    pub fn drop_search_index_for_tests(&mut self) -> Result<(), MemoryError> {
        block_on(async { store::drop_search_index_for_tests(&self.paths.db_path).await })?;
        Ok(())
    }

    fn refresh_cache(&mut self) -> Result<(), MemoryError> {
        self.records = block_on(async { store::load_records(&self.paths.db_path).await })?;
        self.proposals = block_on(async { store::load_proposals(&self.paths.db_path).await })?;
        self.events = block_on(async { store::load_events(&self.paths.db_path).await })?;
        self.edges = block_on(async { store::load_edges(&self.paths.db_path).await })?;
        self.projection_state =
            block_on(async { store::load_projection_state(&self.paths.db_path).await })?;
        Ok(())
    }

    fn acquire_write_guard(&self, operation: &str) -> Result<MemoryWriteGuard, MemoryError> {
        MemoryWriteGuard::acquire(&self.paths.lock_path, operation)
    }

    fn maybe_fail_write_for_tests(&self) -> Result<(), MemoryError> {
        if self.write_should_fail_for_tests {
            return Err(MemoryError("write failure injected for tests".to_string()));
        }

        Ok(())
    }

    fn rebuild_projections_after_commit(&mut self) -> Result<(), MemoryError> {
        let rebuilt_at = now_string();
        let _ = block_on(async { store::rebuild_search_index(&self.paths.db_path).await })?;
        match rebuild_projections(
            &self.paths.core_md,
            &self.paths.status_md,
            &self.paths.permanent_md,
            &self.paths.decisions_md,
            &self.paths.wiki_index_md,
            &self.paths.wiki_core_md,
            &self.paths.wiki_decisions_md,
            &self.paths.wiki_status_md,
            &self.records,
            &rebuilt_at,
            self.projection_should_fail_for_tests,
        ) {
            Ok(_) => {
                self.projection_state = ProjectionState {
                    stale: false,
                    warning: None,
                    last_rebuild_at: Some(rebuilt_at),
                };
                block_on(async {
                    store::save_projection_state(&self.paths.db_path, &self.projection_state).await
                })?;
                Ok(())
            }
            Err(error) => {
                self.projection_state = ProjectionState {
                    stale: true,
                    warning: Some(error),
                    last_rebuild_at: self.projection_state.last_rebuild_at.clone(),
                };
                block_on(async {
                    store::save_projection_state(&self.paths.db_path, &self.projection_state).await
                })?;
                Ok(())
            }
        }
    }

    fn update_status(
        &mut self,
        memory_id: &str,
        expected_version: i64,
        status: MemoryStatus,
        event_kind: MemoryEventKind,
    ) -> Result<(), MemoryError> {
        let record = self
            .records
            .iter()
            .find(|record| record.memory_id == memory_id)
            .cloned()
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {memory_id}")))?;

        let event = MemoryEvent {
            event_id: next_id("evt"),
            event_kind,
            memory_id: Some(memory_id.to_string()),
            proposal_id: None,
            scope: record.scope,
            actor: "user".to_string(),
            reason: Some("status update".to_string()),
            evidence_json: "[]".to_string(),
            created_at: now_string(),
        };

        block_on(async {
            store::update_record_status(
                &self.paths.db_path,
                memory_id,
                expected_version,
                status,
                &event,
            )
            .await
        })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()
    }
}

pub fn default_memory_kernel() -> MemoryKernel {
    let root = std::env::temp_dir().join(format!("aicore-default-memory-{}", next_id("seed")));
    let paths = MemoryPaths::new(root);
    let mut kernel = MemoryKernel::open(paths).expect("default memory kernel should open");
    kernel
        .submit_assistant_summary(
            MemoryScope::GlobalMain {
                instance_id: "global-main".to_string(),
            },
            "User prefers Chinese user-facing interaction.",
        )
        .expect("default proposal should be insertable");
    kernel
}

fn next_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    format!("{prefix}_{nanos}")
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_secs()
        .to_string()
}

fn infer_language(content: &str) -> &'static str {
    if content.is_ascii() { "en" } else { "zh-CN" }
}

fn block_on<F, T>(future: F) -> Result<T, MemoryError>
where
    F: Future<Output = Result<T, MemoryError>>,
{
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| MemoryError(error.to_string()))?
        .block_on(future)
}

pub fn build_core_projection_for_tests(records: &[MemoryRecord]) -> String {
    build_core_projection(records)
}

pub fn build_status_projection_for_tests(records: &[MemoryRecord]) -> String {
    build_status_projection(records)
}

pub fn build_permanent_projection_for_tests(records: &[MemoryRecord]) -> String {
    build_permanent_projection(records)
}

pub fn build_decisions_projection_for_tests(records: &[MemoryRecord]) -> String {
    build_decisions_projection(records)
}
