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
    MemoryKernel, build_core_projection_for_tests, build_decisions_projection_for_tests,
    build_permanent_projection_for_tests, build_status_projection_for_tests, default_memory_kernel,
};
pub use paths::MemoryPaths;
pub use safety::blocks_secret;
pub use search::build_memory_pack_for_tests;
pub use types::{
    MemoryAuditReport, MemoryEdge, MemoryError, MemoryEvent, MemoryEventKind, MemoryPermanence,
    MemoryProposal, MemoryProposalStatus, MemoryRecord, MemoryScope, MemorySource, MemoryStatus,
    MemoryType, ProjectionState, RememberInput, SearchQuery,
};

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use crate::{
        MemoryEventKind, MemoryKernel, MemoryPaths, MemoryPermanence, MemoryScope, MemorySource,
        MemoryStatus, MemoryType, RememberInput, SearchQuery, blocks_secret,
        build_core_projection_for_tests, build_decisions_projection_for_tests,
        build_memory_pack_for_tests, build_permanent_projection_for_tests,
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
    fn remember_initializes_record_version() {
        let mut kernel =
            MemoryKernel::open(temp_paths("remember-version")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "初始版本".to_string(),
                localized_summary: "初始版本".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        assert_eq!(kernel.records()[0].record_version, 1);
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
    fn remember_without_model_normalization_keeps_normalized_language_same_as_content_language() {
        let mut kernel =
            MemoryKernel::open(temp_paths("remember-language")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "中文原文".to_string(),
                localized_summary: "中文原文".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        assert_eq!(kernel.records()[0].content_language, "zh-CN");
        assert_eq!(kernel.records()[0].normalized_content, "中文原文");
        assert_eq!(kernel.records()[0].normalized_language, "zh-CN");
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
    fn assistant_summary_proposal_stores_language_fields() {
        let mut kernel = MemoryKernel::open(temp_paths("assistant-summary-language"))
            .expect("memory kernel should open");

        kernel
            .submit_assistant_summary(global_scope(), "中文 assistant summary proposal")
            .expect("assistant summary should succeed");

        assert_eq!(kernel.proposals()[0].content_language, "zh-CN");
        assert_eq!(
            kernel.proposals()[0].normalized_content,
            "中文 assistant summary proposal"
        );
        assert_eq!(kernel.proposals()[0].normalized_language, "zh-CN");
    }

    #[test]
    fn assistant_summary_writes_proposed_event() {
        let mut kernel = MemoryKernel::open(temp_paths("assistant-summary-event"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_assistant_summary(global_scope(), "assistant summary proposal")
            .expect("assistant summary should succeed");

        assert_eq!(kernel.events().len(), 1);
        assert_eq!(kernel.events()[0].event_kind, MemoryEventKind::Proposed);
        assert_eq!(
            kernel.events()[0].proposal_id.as_deref(),
            Some(proposal_id.as_str())
        );
        assert_eq!(kernel.events()[0].memory_id, None);
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
    fn correct_rejects_stale_expected_version() {
        let mut kernel = MemoryKernel::open(temp_paths("correct-stale-version"))
            .expect("memory kernel should open");

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

        let error = kernel
            .correct_by_user_with_version(&old_id, 0, "新记忆")
            .expect_err("stale version should be rejected");

        assert!(error.0.contains("stale memory version"));
    }

    #[test]
    fn correction_edge_points_from_new_memory_to_old_memory() {
        let mut kernel =
            MemoryKernel::open(temp_paths("correct-edge")).expect("memory kernel should open");

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

        assert!(
            kernel
                .edges()
                .iter()
                .any(|edge| edge.from_memory_id == new_id
                    && edge.to_memory_id == old_id
                    && edge.relation == "supersedes")
        );
    }

    #[test]
    fn correction_reinfers_language_from_new_content() {
        let mut kernel =
            MemoryKernel::open(temp_paths("correct-language")).expect("memory kernel should open");

        let old_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "English memory".to_string(),
                localized_summary: "English memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let new_id = kernel
            .correct_by_user(&old_id, "新的中文纠正")
            .expect("correct should succeed");

        let new_record = kernel
            .records()
            .iter()
            .find(|record| record.memory_id == new_id)
            .expect("new record should exist");

        assert_eq!(new_record.content_language, "zh-CN");
        assert_eq!(new_record.normalized_content, "新的中文纠正");
        assert_eq!(new_record.normalized_language, "zh-CN");
    }

    #[test]
    fn archive_rejects_stale_expected_version() {
        let mut kernel = MemoryKernel::open(temp_paths("archive-stale-version"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待归档记忆".to_string(),
                localized_summary: "待归档记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let error = kernel
            .archive_with_version(&memory_id, 0)
            .expect_err("stale version should be rejected");

        assert!(error.0.contains("stale memory version"));
    }

    #[test]
    fn forget_rejects_stale_expected_version() {
        let mut kernel = MemoryKernel::open(temp_paths("forget-stale-version"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待遗忘记忆".to_string(),
                localized_summary: "待遗忘记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let error = kernel
            .forget_with_version(&memory_id, 0)
            .expect_err("stale version should be rejected");

        assert!(error.0.contains("stale memory version"));
    }

    #[test]
    fn archive_increments_record_version() {
        let mut kernel =
            MemoryKernel::open(temp_paths("archive-version")).expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待归档版本".to_string(),
                localized_summary: "待归档版本".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        kernel
            .archive_with_version(&memory_id, 1)
            .expect("archive should succeed");

        let record = kernel
            .records()
            .iter()
            .find(|record| record.memory_id == memory_id)
            .expect("record should exist");

        assert_eq!(record.record_version, 2);
        assert_eq!(record.status, MemoryStatus::Archived);
    }

    #[test]
    fn correct_creates_new_record_with_version_1() {
        let mut kernel = MemoryKernel::open(temp_paths("correct-new-version"))
            .expect("memory kernel should open");

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
            .correct_by_user_with_version(&old_id, 1, "新记忆")
            .expect("correct should succeed");

        let new_record = kernel
            .records()
            .iter()
            .find(|record| record.memory_id == new_id)
            .expect("new record should exist");

        assert_eq!(new_record.record_version, 1);
    }

    #[test]
    fn superseded_old_record_version_increments() {
        let mut kernel = MemoryKernel::open(temp_paths("correct-old-version"))
            .expect("memory kernel should open");

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

        kernel
            .correct_by_user_with_version(&old_id, 1, "新记忆")
            .expect("correct should succeed");

        let old_record = kernel
            .records()
            .iter()
            .find(|record| record.memory_id == old_id)
            .expect("old record should exist");

        assert_eq!(old_record.record_version, 2);
        assert_eq!(old_record.status, MemoryStatus::Superseded);
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
        assert_eq!(
            kernel.projection_state().warning.as_deref(),
            Some("projection failure injected for tests")
        );
        assert_eq!(kernel.projection_state().last_rebuild_at, None);
    }

    #[test]
    fn successful_rebuild_clears_stale_and_warning() {
        let mut kernel = MemoryKernel::open(temp_paths("projection-recover"))
            .expect("memory kernel should open");
        kernel.set_projection_failure_for_tests(true);

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "第一次失败".to_string(),
                localized_summary: "第一次失败".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should still succeed");

        assert!(kernel.projection_state().stale);

        kernel.set_projection_failure_for_tests(false);
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "第二次成功".to_string(),
                localized_summary: "第二次成功".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        assert!(!kernel.projection_state().stale);
        assert_eq!(kernel.projection_state().warning, None);
        assert!(kernel.projection_state().last_rebuild_at.is_some());
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
    fn permanent_projection_contains_only_active_permanent_records() {
        let mut kernel = MemoryKernel::open(temp_paths("permanent-projection"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "长期有效规则".to_string(),
                localized_summary: "长期有效规则".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "普通规则".to_string(),
                localized_summary: "普通规则".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let permanent = build_permanent_projection_for_tests(kernel.records());
        assert!(permanent.contains("长期有效规则"));
        assert!(!permanent.contains("普通规则"));
    }

    #[test]
    fn archived_permanent_record_is_excluded_from_permanent_projection() {
        let mut kernel = MemoryKernel::open(temp_paths("permanent-archived"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "归档的长期规则".to_string(),
                localized_summary: "归档的长期规则".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .archive_with_version(&memory_id, 1)
            .expect("archive should succeed");

        let permanent = build_permanent_projection_for_tests(kernel.records());
        assert!(!permanent.contains("归档的长期规则"));
    }

    #[test]
    fn decisions_projection_contains_active_decision_records() {
        let mut kernel = MemoryKernel::open(temp_paths("decisions-projection"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Decision,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "决定优先 CLI 再做 TUI".to_string(),
                localized_summary: "优先 CLI".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let decisions = build_decisions_projection_for_tests(kernel.records());
        assert!(decisions.contains("决定优先 CLI 再做 TUI"));
    }

    #[test]
    fn decisions_projection_excludes_archived_decisions() {
        let mut kernel = MemoryKernel::open(temp_paths("decisions-archived"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Decision,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "已归档决定".to_string(),
                localized_summary: "已归档决定".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .archive_with_version(&memory_id, 1)
            .expect("archive should succeed");

        let decisions = build_decisions_projection_for_tests(kernel.records());
        assert!(!decisions.contains("已归档决定"));
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

    #[test]
    fn audit_passes_for_remembered_memory() {
        let mut kernel =
            MemoryKernel::open(temp_paths("audit-remember")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "审计记忆".to_string(),
                localized_summary: "审计记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(report.ok);
        assert_eq!(report.checked_events, 1);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn audit_detects_accepted_event_without_record() {
        let mut kernel = MemoryKernel::open(temp_paths("audit-missing-record"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待删记录".to_string(),
                localized_summary: "待删记录".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .delete_record_for_tests(&memory_id)
            .expect("record deletion should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("accepted event") && issue.contains("missing memory"))
        );
    }

    #[test]
    fn audit_passes_for_assistant_summary_proposal() {
        let mut kernel =
            MemoryKernel::open(temp_paths("audit-proposal")).expect("memory kernel should open");

        kernel
            .submit_assistant_summary(global_scope(), "assistant summary proposal")
            .expect("assistant summary should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(report.ok);
        assert_eq!(report.checked_events, 1);
    }

    #[test]
    fn audit_detects_proposed_event_without_proposal() {
        let mut kernel = MemoryKernel::open(temp_paths("audit-missing-proposal"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_assistant_summary(global_scope(), "assistant summary proposal")
            .expect("assistant summary should succeed");
        kernel
            .delete_proposal_for_tests(&proposal_id)
            .expect("proposal deletion should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(!report.ok);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("proposed event") && issue.contains("missing proposal")));
    }

    #[test]
    fn audit_passes_for_user_correction_supersedes_edge() {
        let mut kernel =
            MemoryKernel::open(temp_paths("audit-correction")).expect("memory kernel should open");

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
        kernel
            .correct_by_user_with_version(&old_id, 1, "新记忆")
            .expect("correct should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(report.ok);
    }

    #[test]
    fn audit_detects_missing_supersedes_edge_for_correction() {
        let mut kernel = MemoryKernel::open(temp_paths("audit-missing-edge"))
            .expect("memory kernel should open");

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
            .correct_by_user_with_version(&old_id, 1, "新记忆")
            .expect("correct should succeed");
        kernel
            .delete_edge_for_tests(&new_id, &old_id, "supersedes")
            .expect("edge deletion should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("missing supersedes edge"))
        );
    }

    #[test]
    fn audit_passes_for_archived_memory() {
        let mut kernel =
            MemoryKernel::open(temp_paths("audit-archived")).expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待归档".to_string(),
                localized_summary: "待归档".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .archive_with_version(&memory_id, 1)
            .expect("archive should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(report.ok);
    }

    #[test]
    fn audit_detects_archived_event_without_archived_record() {
        let mut kernel = MemoryKernel::open(temp_paths("audit-archived-mismatch"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待归档".to_string(),
                localized_summary: "待归档".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .archive_with_version(&memory_id, 1)
            .expect("archive should succeed");
        kernel
            .force_record_status_for_tests(&memory_id, MemoryStatus::Active)
            .expect("status override should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("archived event") && issue.contains("non-archived"))
        );
    }

    #[test]
    fn audit_passes_for_forgotten_memory() {
        let mut kernel =
            MemoryKernel::open(temp_paths("audit-forgotten")).expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待遗忘".to_string(),
                localized_summary: "待遗忘".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .forget_with_version(&memory_id, 1)
            .expect("forget should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(report.ok);
    }

    #[test]
    fn audit_detects_forgotten_event_without_forgotten_record() {
        let mut kernel = MemoryKernel::open(temp_paths("audit-forgotten-mismatch"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "待遗忘".to_string(),
                localized_summary: "待遗忘".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .forget_with_version(&memory_id, 1)
            .expect("forget should succeed");
        kernel
            .force_record_status_for_tests(&memory_id, MemoryStatus::Active)
            .expect("status override should succeed");

        let report = kernel.verify_ledger_consistency();
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("forgotten event") && issue.contains("non-forgotten"))
        );
    }
}
