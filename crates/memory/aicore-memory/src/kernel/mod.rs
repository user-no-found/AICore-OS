mod audit;
mod proposals;
mod remember;
mod search;
mod wiki;

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
