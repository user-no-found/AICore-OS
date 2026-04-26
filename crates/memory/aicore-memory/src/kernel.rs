use std::{
    fs,
    future::Future,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::runtime::Builder;

use crate::{
    ids::{MemoryId, MemoryProposalId},
    paths::MemoryPaths,
    projection::{build_core_projection, build_status_projection, rebuild_projections},
    search::filter_records,
    store,
    types::{
        MemoryError, MemoryEvent, MemoryEventKind, MemoryProposal, MemoryProposalStatus,
        MemoryRecord, MemoryScope, MemorySource, MemoryStatus, MemoryType, ProjectionState,
        RememberInput, SearchQuery,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryKernel {
    paths: MemoryPaths,
    records: Vec<MemoryRecord>,
    proposals: Vec<MemoryProposal>,
    events: Vec<MemoryEvent>,
    projection_state: ProjectionState,
    projection_should_fail_for_tests: bool,
}

impl MemoryKernel {
    pub fn open(paths: MemoryPaths) -> Result<Self, MemoryError> {
        fs::create_dir_all(&paths.root).map_err(|error| MemoryError(error.to_string()))?;
        fs::create_dir_all(&paths.projections_dir).map_err(|error| MemoryError(error.to_string()))?;

        block_on(async { store::init_schema(&paths.db_path).await })?;

        let mut kernel = Self {
            paths,
            records: Vec::new(),
            proposals: Vec::new(),
            events: Vec::new(),
            projection_state: ProjectionState {
                stale: false,
                warning: None,
            },
            projection_should_fail_for_tests: false,
        };

        kernel.refresh_cache()?;
        Ok(kernel)
    }

    pub fn table_names(&self) -> Result<Vec<String>, MemoryError> {
        block_on(async { store::table_names(&self.paths.db_path).await })
    }

    pub fn remember_user_explicit(&mut self, input: RememberInput) -> Result<MemoryId, MemoryError> {
        let timestamp = now_string();
        let memory_id = next_id("mem");
        let event_id = next_id("evt");
        let normalized = normalize(&input.content);

        let record = MemoryRecord {
            memory_id: memory_id.clone(),
            memory_type: input.memory_type,
            status: MemoryStatus::Active,
            permanence: input.permanence,
            scope: input.scope.clone(),
            content: input.content,
            content_language: "zh".to_string(),
            normalized_content: normalized,
            normalized_language: "en".to_string(),
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

        block_on(async { store::insert_record_and_event(&self.paths.db_path, &record, &event).await })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()?;

        Ok(memory_id)
    }

    pub fn submit_assistant_summary(
        &mut self,
        scope: MemoryScope,
        content: &str,
    ) -> Result<MemoryProposalId, MemoryError> {
        let proposal = MemoryProposal {
            proposal_id: next_id("prop"),
            memory_type: MemoryType::Working,
            scope,
            source: MemorySource::AssistantSummary,
            status: MemoryProposalStatus::Open,
            content: content.to_string(),
            normalized_content: normalize(content),
            localized_summary: content.to_string(),
            created_at: now_string(),
        };

        block_on(async { store::insert_proposal(&self.paths.db_path, &proposal).await })?;
        self.refresh_cache()?;
        Ok(proposal.proposal_id.clone())
    }

    pub fn correct_by_user(
        &mut self,
        old_memory_id: &str,
        new_content: &str,
    ) -> Result<MemoryId, MemoryError> {
        let old_record = self
            .records
            .iter()
            .find(|record| record.memory_id == old_memory_id)
            .cloned()
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {old_memory_id}")))?;

        let timestamp = now_string();
        let new_memory_id = next_id("mem");
        let record = MemoryRecord {
            memory_id: new_memory_id.clone(),
            memory_type: old_record.memory_type,
            status: MemoryStatus::Active,
            permanence: old_record.permanence,
            scope: old_record.scope.clone(),
            content: new_content.to_string(),
            content_language: old_record.content_language,
            normalized_content: normalize(new_content),
            normalized_language: old_record.normalized_language,
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
            store::supersede_record(&self.paths.db_path, old_memory_id, &record, &event).await
        })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()?;

        Ok(new_memory_id)
    }

    pub fn archive(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        self.update_status(memory_id, MemoryStatus::Archived, MemoryEventKind::Archived)
    }

    pub fn forget(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        self.update_status(memory_id, MemoryStatus::Forgotten, MemoryEventKind::Forgotten)
    }

    pub fn search(&self, query: SearchQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        Ok(filter_records(&self.records, &query))
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

    pub fn projection_state(&self) -> &ProjectionState {
        &self.projection_state
    }

    pub fn core_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.core_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn status_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.status_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn set_projection_failure_for_tests(&mut self, should_fail: bool) {
        self.projection_should_fail_for_tests = should_fail;
    }

    fn refresh_cache(&mut self) -> Result<(), MemoryError> {
        self.records = block_on(async { store::load_records(&self.paths.db_path).await })?;
        self.proposals = block_on(async { store::load_proposals(&self.paths.db_path).await })?;
        self.events = block_on(async { store::load_events(&self.paths.db_path).await })?;
        self.projection_state =
            block_on(async { store::load_projection_state(&self.paths.db_path).await })?;
        Ok(())
    }

    fn rebuild_projections_after_commit(&mut self) -> Result<(), MemoryError> {
        match rebuild_projections(
            &self.paths.core_md,
            &self.paths.status_md,
            &self.records,
            self.projection_should_fail_for_tests,
        ) {
            Ok(_) => {
                self.projection_state = ProjectionState {
                    stale: false,
                    warning: None,
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

        block_on(async { store::update_record_status(&self.paths.db_path, memory_id, status, &event).await })?;
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

fn normalize(content: &str) -> String {
    content.to_ascii_lowercase()
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
