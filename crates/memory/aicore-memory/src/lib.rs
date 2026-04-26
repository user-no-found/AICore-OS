mod ids;
mod kernel;
mod paths;
mod projection;
mod safety;
mod search;
mod store;
mod types;

pub use ids::{MemoryEventId, MemoryId, MemoryProposalId, MemorySnapshotRev};
pub use kernel::{
    MemoryKernel, build_core_projection_for_tests, build_status_projection_for_tests,
    default_memory_kernel,
};
pub use paths::MemoryPaths;
pub use safety::blocks_secret;
pub use search::build_memory_pack_for_tests;
pub use types::{
    MemoryError, MemoryEvent, MemoryEventKind, MemoryPermanence, MemoryProposal,
    MemoryProposalStatus, MemoryRecord, MemoryScope, MemorySource, MemoryStatus, MemoryType,
    ProjectionState, RememberInput, SearchQuery,
};

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use crate::{
        MemoryEventKind, MemoryKernel, MemoryPaths, MemoryPermanence, MemoryScope, MemorySource,
        MemoryStatus, MemoryType, RememberInput, SearchQuery, blocks_secret,
        build_core_projection_for_tests, build_memory_pack_for_tests,
        build_status_projection_for_tests,
    };

    fn temp_paths(name: &str) -> MemoryPaths {
        let root = env::temp_dir().join(format!("aicore-memory-tests-{name}"));
        if root.exists() {
            fs::remove_dir_all(&root).expect("temp memory root should be removable");
        }
        MemoryPaths::new(root)
    }

    fn global_scope() -> MemoryScope {
        MemoryScope::GlobalMain {
            instance_id: "global-main".to_string(),
        }
    }

    #[test]
    fn memory_db_initializes_expected_tables() {
        let kernel = MemoryKernel::open(temp_paths("init")).expect("memory kernel should open");
        let tables = kernel.table_names().expect("tables should be listable");

        assert!(tables.contains(&"memory_records".to_string()));
        assert!(tables.contains(&"memory_events".to_string()));
        assert!(tables.contains(&"memory_proposals".to_string()));
        assert!(tables.contains(&"memory_edges".to_string()));
        assert!(tables.contains(&"memory_snapshots".to_string()));
        assert!(tables.contains(&"memory_projection_state".to_string()));
    }

    #[test]
    fn user_explicit_remember_creates_active_record_immediately() {
        let mut kernel =
            MemoryKernel::open(temp_paths("remember-active")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "记住这个核心偏好".to_string(),
                localized_summary: "核心偏好".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        assert_eq!(kernel.records().len(), 1);
        assert_eq!(kernel.records()[0].status, MemoryStatus::Active);
    }

    #[test]
    fn remember_writes_accepted_memory_event() {
        let mut kernel =
            MemoryKernel::open(temp_paths("remember-event")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "写入 accepted event".to_string(),
                localized_summary: "accepted event".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        assert_eq!(kernel.events().len(), 1);
        assert_eq!(kernel.events()[0].event_kind, MemoryEventKind::Accepted);
        assert_eq!(kernel.records()[0].source, MemorySource::UserExplicit);
    }

    #[test]
    fn assistant_summary_creates_proposal_not_record() {
        let mut kernel =
            MemoryKernel::open(temp_paths("assistant-summary")).expect("memory kernel should open");

        kernel
            .submit_assistant_summary(global_scope(), "assistant summary proposal")
            .expect("assistant summary should succeed");

        assert!(kernel.records().is_empty());
        assert_eq!(kernel.proposals().len(), 1);
    }

    #[test]
    fn correction_by_user_supersedes_old_memory() {
        let mut kernel =
            MemoryKernel::open(temp_paths("correct")).expect("memory kernel should open");

        let old_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "旧记忆".to_string(),
                localized_summary: "旧记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let new_id = kernel
            .correct_by_user(&old_id, "新记忆")
            .expect("correct should succeed");

        assert_ne!(old_id, new_id);
        assert_eq!(kernel.records().len(), 2);
        assert!(
            kernel
                .records()
                .iter()
                .any(|record| record.memory_id == old_id
                    && record.status == MemoryStatus::Superseded)
        );
        assert!(
            kernel
                .records()
                .iter()
                .any(|record| record.memory_id == new_id && record.status == MemoryStatus::Active)
        );
    }

    #[test]
    fn archived_permanent_memory_is_not_returned_by_default_search() {
        let mut kernel =
            MemoryKernel::open(temp_paths("archive-search")).expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "永久记忆".to_string(),
                localized_summary: "永久记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel.archive(&memory_id).expect("archive should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "永久".to_string(),
                scope: None,
            })
            .expect("search should succeed");

        assert!(results.is_empty());
    }

    #[test]
    fn search_uses_scope_filter() {
        let mut kernel =
            MemoryKernel::open(temp_paths("scope-filter")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "全局记忆".to_string(),
                localized_summary: "全局记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: MemoryScope::Workspace {
                    instance_id: "ws-demo".to_string(),
                    workspace_root: "/tmp/ws-demo".to_string(),
                },
                content: "工作区记忆".to_string(),
                localized_summary: "工作区记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "记忆".to_string(),
                scope: Some(MemoryScope::Workspace {
                    instance_id: "ws-demo".to_string(),
                    workspace_root: "/tmp/ws-demo".to_string(),
                }),
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "工作区记忆");
    }

    #[test]
    fn search_uses_content_and_normalized_content() {
        let mut kernel =
            MemoryKernel::open(temp_paths("normalized-search")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "AICore Memory".to_string(),
                localized_summary: "aicore memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "aicore".to_string(),
                scope: None,
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn projection_rebuild_failure_marks_stale_not_rollback_db() {
        let mut kernel =
            MemoryKernel::open(temp_paths("projection-stale")).expect("memory kernel should open");
        kernel.set_projection_failure_for_tests(true);

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "投影失败".to_string(),
                localized_summary: "投影失败".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should still succeed");

        assert_eq!(kernel.records().len(), 1);
        assert!(kernel.projection_state().stale);
    }

    #[test]
    fn core_projection_contains_active_core_records() {
        let mut kernel =
            MemoryKernel::open(temp_paths("core-projection")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "核心规则".to_string(),
                localized_summary: "核心规则".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "工作记忆".to_string(),
                localized_summary: "工作记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let core = build_core_projection_for_tests(kernel.records());
        assert!(core.contains("核心规则"));
        assert!(!core.contains("工作记忆"));
    }

    #[test]
    fn status_projection_contains_current_stage() {
        let mut kernel =
            MemoryKernel::open(temp_paths("status-projection")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Status,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "当前阶段".to_string(),
                localized_summary: "当前阶段".to_string(),
                state_key: Some("stage".to_string()),
                current_state: Some("P6.1".to_string()),
            })
            .expect("remember should succeed");

        let status = build_status_projection_for_tests(kernel.records());
        assert!(status.contains("P6.1"));
    }

    #[test]
    fn memory_pack_respects_token_budget() {
        let mut kernel =
            MemoryKernel::open(temp_paths("memory-pack")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "非常重要的长期记忆".to_string(),
                localized_summary: "重要记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "次要工作记忆".to_string(),
                localized_summary: "次要工作记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let pack = build_memory_pack_for_tests(kernel.records(), 8);

        assert_eq!(pack.len(), 1);
        assert_eq!(pack[0].content, "非常重要的长期记忆");
    }

    #[test]
    fn stage_status_is_not_current_instruction() {
        let mut kernel = MemoryKernel::open(temp_paths("status-not-instruction"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "始终使用中文回复".to_string(),
                localized_summary: "中文回复".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Status,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "当前阶段".to_string(),
                localized_summary: "当前阶段".to_string(),
                state_key: Some("stage".to_string()),
                current_state: Some("P6.1".to_string()),
            })
            .expect("remember should succeed");

        let status = build_status_projection_for_tests(kernel.records());
        assert!(status.contains("P6.1"));
        assert!(!status.contains("始终使用中文回复"));
    }

    #[test]
    fn safety_scan_blocks_secret_but_not_technical_discussion() {
        assert!(blocks_secret("api_key=sk-test-secret"));
        assert!(!blocks_secret(
            "这里讨论 api_key 命名规范和 secret storage 设计"
        ));
    }
}
