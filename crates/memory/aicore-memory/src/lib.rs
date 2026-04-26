mod agent;
mod ids;
mod kernel;
mod lock;
mod paths;
mod projection;
mod safety;
mod search;
mod store;
mod types;

pub use agent::RuleBasedMemoryAgent;
pub use ids::{MemoryEventId, MemoryId, MemoryProposalId, MemorySnapshotRev};
pub use kernel::{
    MemoryKernel, build_core_projection_for_tests, build_decisions_projection_for_tests,
    build_permanent_projection_for_tests, build_status_projection_for_tests, default_memory_kernel,
};
pub use paths::MemoryPaths;
pub use safety::blocks_secret;
pub use search::build_memory_pack_for_tests;
pub use types::{
    MemoryAgentOutput, MemoryAuditReport, MemoryEdge, MemoryError, MemoryEvent, MemoryEventKind,
    MemoryPermanence, MemoryProposal, MemoryProposalStatus, MemoryRecord, MemoryRequestedOutput,
    MemoryScope, MemorySource, MemoryStatus, MemoryTrigger, MemoryType, MemoryWorkBatch,
    ProjectionState, RememberInput, SearchQuery, SearchResult,
};

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, env, fs, thread};

    use crate::{
        MemoryAgentOutput, MemoryEventKind, MemoryKernel, MemoryPaths, MemoryPermanence,
        MemoryProposal, MemoryProposalStatus, MemoryRequestedOutput, MemoryScope, MemorySource,
        MemoryStatus, MemoryTrigger, MemoryType, MemoryWorkBatch, RememberInput,
        RuleBasedMemoryAgent, SearchQuery, blocks_secret, build_core_projection_for_tests,
        build_decisions_projection_for_tests, build_memory_pack_for_tests,
        build_permanent_projection_for_tests, build_status_projection_for_tests,
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

    fn write_lock_file(paths: &MemoryPaths, created_at: &str, operation: &str) {
        fs::create_dir_all(&paths.root).expect("memory root should be creatable");
        fs::write(
            &paths.lock_path,
            format!("pid=999999\ncreated_at={created_at}\noperation={operation}\n"),
        )
        .expect("lock file should be writable");
    }

    fn work_batch(trigger: MemoryTrigger, excerpts: Vec<&str>) -> MemoryWorkBatch {
        MemoryWorkBatch {
            instance_id: "global-main".to_string(),
            scope: global_scope(),
            trigger,
            recent_events_summary: String::new(),
            raw_excerpts: excerpts.into_iter().map(ToString::to_string).collect(),
            existing_memory_hits: Vec::new(),
            token_budget: 1024,
            requested_outputs: vec![MemoryRequestedOutput::Proposals],
        }
    }

    fn agent_proposal(memory_type: MemoryType, content: &str) -> MemoryProposal {
        MemoryProposal {
            proposal_id: format!("agent_prop_{content}"),
            memory_type,
            scope: global_scope(),
            source: MemorySource::RuleBasedAgent,
            status: MemoryProposalStatus::Rejected,
            content: content.to_string(),
            content_language: if content.is_ascii() {
                "en".to_string()
            } else {
                "zh-CN".to_string()
            },
            normalized_content: content.to_string(),
            normalized_language: if content.is_ascii() {
                "en".to_string()
            } else {
                "zh-CN".to_string()
            },
            localized_summary: content.to_string(),
            created_at: "0".to_string(),
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
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
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
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.content, "工作区记忆");
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
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_filters_by_memory_type() {
        let mut kernel = MemoryKernel::open(temp_paths("search-filter-type"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "共享关键词".to_string(),
                localized_summary: "共享关键词".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Decision,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "共享关键词".to_string(),
                localized_summary: "共享关键词".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "共享".to_string(),
                scope: None,
                memory_type: Some(MemoryType::Decision),
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.memory_type, MemoryType::Decision);
    }

    #[test]
    fn search_filters_by_source() {
        let mut kernel = MemoryKernel::open(temp_paths("search-filter-source"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "共享来源".to_string(),
                localized_summary: "共享来源".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .submit_assistant_summary(global_scope(), "共享来源")
            .expect("assistant summary should succeed");
        let proposal_id = kernel.proposals()[0].proposal_id.clone();
        kernel
            .accept_proposal(&proposal_id, "user", Some("接受"))
            .expect("accept should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "共享".to_string(),
                scope: None,
                memory_type: None,
                source: Some(MemorySource::AssistantSummary),
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.source, MemorySource::AssistantSummary);
    }

    #[test]
    fn search_filters_by_permanence() {
        let mut kernel = MemoryKernel::open(temp_paths("search-filter-permanence"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "共享永久性".to_string(),
                localized_summary: "共享永久性".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "共享永久性".to_string(),
                localized_summary: "共享永久性".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "共享".to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: Some(MemoryPermanence::Permanent),
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.permanence, MemoryPermanence::Permanent);
    }

    #[test]
    fn search_respects_limit() {
        let mut kernel =
            MemoryKernel::open(temp_paths("search-limit")).expect("memory kernel should open");

        for index in 0..3 {
            kernel
                .remember_user_explicit(RememberInput {
                    memory_type: MemoryType::Core,
                    permanence: MemoryPermanence::Standard,
                    scope: global_scope(),
                    content: format!("limit test {index}"),
                    localized_summary: format!("limit test {index}"),
                    state_key: None,
                    current_state: None,
                })
                .expect("remember should succeed");
        }

        let results = kernel
            .search(SearchQuery {
                text: "limit".to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: None,
                limit: Some(2),
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_returns_score_and_matched_fields() {
        let mut kernel =
            MemoryKernel::open(temp_paths("search-score")).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "匹配内容".to_string(),
                localized_summary: "匹配摘要".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "匹配".to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0);
        assert!(!results[0].matched_fields.is_empty());
    }

    #[test]
    fn search_prioritizes_localized_summary_match() {
        let mut kernel = MemoryKernel::open(temp_paths("search-summary-priority"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "普通内容".to_string(),
                localized_summary: "关键摘要".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "关键摘要".to_string(),
                localized_summary: "普通摘要".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "关键".to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results[0].record.localized_summary, "关键摘要");
        assert!(
            results[0]
                .matched_fields
                .contains(&"localized_summary".to_string())
        );
    }

    #[test]
    fn search_prioritizes_content_over_normalized_content() {
        let mut kernel = MemoryKernel::open(temp_paths("search-content-priority"))
            .expect("memory kernel should open");

        let first_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "关键字在内容".to_string(),
                localized_summary: "普通摘要".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        let second_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "普通内容".to_string(),
                localized_summary: "普通摘要".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .force_normalized_content_for_tests(&first_id, "普通规范化")
            .expect("normalized content should be mutable in tests");
        kernel
            .force_normalized_content_for_tests(&second_id, "关键字在规范化")
            .expect("normalized content should be mutable in tests");

        let results = kernel
            .search(SearchQuery {
                text: "关键字".to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results[0].record.content, "关键字在内容");
        assert!(results[0].matched_fields.contains(&"content".to_string()));
    }

    #[test]
    fn search_prioritizes_permanent_core_records() {
        let mut kernel = MemoryKernel::open(temp_paths("search-permanent-core-priority"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "共同关键词".to_string(),
                localized_summary: "共同关键词".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "共同关键词".to_string(),
                localized_summary: "共同关键词".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "共同".to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert_eq!(results[0].record.memory_type, MemoryType::Core);
        assert_eq!(results[0].record.permanence, MemoryPermanence::Permanent);
    }

    #[test]
    fn search_excludes_archived_even_if_permanent() {
        let mut kernel = MemoryKernel::open(temp_paths("search-excludes-archived-permanent"))
            .expect("memory kernel should open");

        let memory_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "归档也不该出现".to_string(),
                localized_summary: "归档也不该出现".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel.archive(&memory_id).expect("archive should succeed");

        let results = kernel
            .search(SearchQuery {
                text: "归档".to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");

        assert!(results.is_empty());
    }

    #[test]
    fn memory_pack_uses_hardened_search_order() {
        let mut kernel = MemoryKernel::open(temp_paths("memory-pack-hardened-order"))
            .expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "普通工作记忆".to_string(),
                localized_summary: "普通工作记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Decision,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "关键决策记忆".to_string(),
                localized_summary: "关键决策记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let pack = build_memory_pack_for_tests(kernel.records(), 64);

        assert_eq!(pack.len(), 2);
        assert_eq!(pack[0].content, "关键决策记忆");
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

    #[test]
    fn manual_lock_blocks_second_writer() {
        let paths = temp_paths("manual-lock-blocks");
        write_lock_file(&paths, "9999999999", "remember_user_explicit");
        let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

        let error = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "被锁阻塞".to_string(),
                localized_summary: "被锁阻塞".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect_err("active lock should block writer");

        assert!(error.0.contains("memory write locked"));
    }

    #[test]
    fn stale_manual_lock_allows_recovery() {
        let paths = temp_paths("stale-lock-recovery");
        write_lock_file(&paths, "0", "remember_user_explicit");
        let lock_path = paths.lock_path.clone();
        let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "stale lock recovery".to_string(),
                localized_summary: "stale lock recovery".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("stale lock should be recoverable");

        assert!(!lock_path.exists());
        assert_eq!(kernel.records().len(), 1);
    }

    #[test]
    fn write_lock_is_released_after_projection_failure() {
        let paths = temp_paths("lock-release-projection-failure");
        let lock_path = paths.lock_path.clone();
        let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
        kernel.set_projection_failure_for_tests(true);

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "projection failure".to_string(),
                localized_summary: "projection failure".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("write should still succeed");

        assert!(!lock_path.exists());
    }

    #[test]
    fn write_lock_is_released_after_storage_error() {
        let paths = temp_paths("lock-release-storage-failure");
        let lock_path = paths.lock_path.clone();
        let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
        kernel.set_write_failure_for_tests(true);

        let error = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "storage failure".to_string(),
                localized_summary: "storage failure".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect_err("injected write failure should fail");

        assert!(error.0.contains("write failure injected for tests"));
        assert!(!lock_path.exists());
    }

    #[test]
    fn concurrent_remember_calls_do_not_reuse_ids() {
        let paths = temp_paths("concurrent-remember");
        let mut handles = Vec::new();

        for index in 0..8 {
            let paths = paths.clone();
            handles.push(thread::spawn(move || {
                let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
                kernel
                    .remember_user_explicit(RememberInput {
                        memory_type: MemoryType::Core,
                        permanence: MemoryPermanence::Standard,
                        scope: global_scope(),
                        content: format!("concurrent remember {index}"),
                        localized_summary: format!("concurrent remember {index}"),
                        state_key: None,
                        current_state: None,
                    })
                    .expect("remember should succeed")
            }));
        }

        let mut ids = Vec::new();
        for handle in handles {
            ids.push(handle.join().expect("thread should finish"));
        }

        let unique: HashSet<_> = ids.iter().cloned().collect();
        assert_eq!(ids.len(), unique.len());

        let kernel = MemoryKernel::open(paths).expect("memory kernel should reopen");
        assert_eq!(kernel.records().len(), ids.len());
    }

    #[test]
    fn memory_audit_passes_after_concurrent_writes() {
        let paths = temp_paths("concurrent-audit");
        let mut handles = Vec::new();

        for index in 0..6 {
            let paths = paths.clone();
            handles.push(thread::spawn(move || {
                let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
                kernel
                    .remember_user_explicit(RememberInput {
                        memory_type: MemoryType::Core,
                        permanence: MemoryPermanence::Standard,
                        scope: global_scope(),
                        content: format!("audit concurrent {index}"),
                        localized_summary: format!("audit concurrent {index}"),
                        state_key: None,
                        current_state: None,
                    })
                    .expect("remember should succeed");
            }));
        }

        for handle in handles {
            handle.join().expect("thread should finish");
        }

        let kernel = MemoryKernel::open(paths).expect("memory kernel should reopen");
        let report = kernel.verify_ledger_consistency();
        assert!(report.ok);
        assert_eq!(report.checked_events, 6);
    }

    #[test]
    fn rule_based_agent_extracts_remember_proposal() {
        let output = RuleBasedMemoryAgent::analyze(&work_batch(
            MemoryTrigger::ExplicitRemember,
            vec!["记住：TUI 是类似 Codex 的终端 AI 编程界面"],
        ));

        assert_eq!(output.proposals.len(), 1);
        assert_eq!(output.proposals[0].memory_type, MemoryType::Core);
        assert_eq!(
            output.proposals[0].content,
            "TUI 是类似 Codex 的终端 AI 编程界面"
        );
    }

    #[test]
    fn rule_based_agent_extracts_stage_status_proposal() {
        let output = RuleBasedMemoryAgent::analyze(&work_batch(
            MemoryTrigger::StageCompleted,
            vec!["已完成 P6.2.4 Memory Lock / Single Writer Guard"],
        ));

        assert_eq!(output.proposals.len(), 1);
        assert_eq!(output.proposals[0].memory_type, MemoryType::Status);
        assert!(
            output.proposals[0]
                .content
                .contains("已完成 P6.2.4 Memory Lock / Single Writer Guard")
        );
    }

    #[test]
    fn rule_based_agent_outputs_proposals_only() {
        let output = RuleBasedMemoryAgent::analyze(&work_batch(
            MemoryTrigger::Correction,
            vec!["纠正：上一条记忆不准确"],
        ));

        assert!(!output.proposals.is_empty());
        assert!(output.corrections.is_empty());
        assert!(output.archive_suggestions.is_empty());
    }

    #[test]
    fn memory_agent_does_not_create_records() {
        let kernel =
            MemoryKernel::open(temp_paths("agent-no-records")).expect("memory kernel should open");
        let before = kernel.records().len();

        let _ = RuleBasedMemoryAgent::analyze(&work_batch(
            MemoryTrigger::ExplicitRemember,
            vec!["记住：不要直接写 record"],
        ));

        assert_eq!(kernel.records().len(), before);
    }

    #[test]
    fn memory_agent_does_not_accept_proposals() {
        let output = RuleBasedMemoryAgent::analyze(&work_batch(
            MemoryTrigger::ExplicitRemember,
            vec!["记住：proposal 不能自动 accept"],
        ));

        assert!(
            output
                .proposals
                .iter()
                .all(|proposal| proposal.status == MemoryProposalStatus::Open)
        );
    }

    #[test]
    fn proposal_dedupe_merges_same_content() {
        let output = RuleBasedMemoryAgent::analyze(&work_batch(
            MemoryTrigger::ExplicitRemember,
            vec!["记住：统一术语", "记住：统一术语"],
        ));

        assert_eq!(output.proposals.len(), 1);
    }

    #[test]
    fn proposal_dedupe_keeps_different_memory_types() {
        let output = RuleBasedMemoryAgent::analyze(&MemoryWorkBatch {
            instance_id: "global-main".to_string(),
            scope: global_scope(),
            trigger: MemoryTrigger::SessionClosed,
            recent_events_summary: String::new(),
            raw_excerpts: vec!["记住：统一术语".to_string(), "已完成 P6.2".to_string()],
            existing_memory_hits: Vec::new(),
            token_budget: 1024,
            requested_outputs: vec![MemoryRequestedOutput::Proposals],
        });

        assert_eq!(output.proposals.len(), 2);
        assert!(
            output
                .proposals
                .iter()
                .any(|proposal| proposal.memory_type == MemoryType::Core)
        );
        assert!(
            output
                .proposals
                .iter()
                .any(|proposal| proposal.memory_type == MemoryType::Status)
        );
    }

    #[test]
    fn submit_agent_output_stores_open_proposals() {
        let mut kernel =
            MemoryKernel::open(temp_paths("agent-intake-open")).expect("memory kernel should open");

        let inserted = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "记住这个提案")],
                corrections: vec!["ignored".to_string()],
                archive_suggestions: vec!["ignored".to_string()],
            })
            .expect("agent output should be stored");

        assert_eq!(inserted.len(), 1);
        assert_eq!(kernel.proposals().len(), 1);
        assert_eq!(kernel.proposals()[0].status, MemoryProposalStatus::Open);
    }

    #[test]
    fn submit_agent_output_does_not_create_records() {
        let mut kernel = MemoryKernel::open(temp_paths("agent-intake-no-records"))
            .expect("memory kernel should open");

        kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "不要创建 record")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored");

        assert!(kernel.records().is_empty());
    }

    #[test]
    fn submit_agent_output_writes_proposed_events() {
        let mut kernel = MemoryKernel::open(temp_paths("agent-intake-events"))
            .expect("memory kernel should open");

        let inserted = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Status, "已完成 P6.3.1")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored");

        assert_eq!(kernel.events().len(), 1);
        assert_eq!(kernel.events()[0].event_kind, MemoryEventKind::Proposed);
        assert_eq!(
            kernel.events()[0].proposal_id.as_deref(),
            Some(inserted[0].as_str())
        );
    }

    #[test]
    fn submit_agent_output_dedupes_existing_open_proposals() {
        let mut kernel = MemoryKernel::open(temp_paths("agent-intake-dedupe"))
            .expect("memory kernel should open");

        let first = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "重复提案")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("first intake should succeed");
        let second = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "重复提案")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("second intake should succeed");

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(kernel.proposals().len(), 1);
    }

    #[test]
    fn submit_agent_output_keeps_different_memory_types() {
        let mut kernel = MemoryKernel::open(temp_paths("agent-intake-types"))
            .expect("memory kernel should open");

        let inserted = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![
                    agent_proposal(MemoryType::Core, "同内容"),
                    agent_proposal(MemoryType::Working, "同内容"),
                ],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored");

        assert_eq!(inserted.len(), 2);
        assert_eq!(kernel.proposals().len(), 2);
    }

    #[test]
    fn submit_agent_output_preserves_language_fields() {
        let mut kernel = MemoryKernel::open(temp_paths("agent-intake-language"))
            .expect("memory kernel should open");

        kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "中文提案")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored");

        let proposal = &kernel.proposals()[0];
        assert_eq!(proposal.content_language, "zh-CN");
        assert_eq!(proposal.normalized_content, "中文提案");
        assert_eq!(proposal.normalized_language, "zh-CN");
    }

    #[test]
    fn submit_agent_output_reassigns_stable_kernel_proposal_ids() {
        let mut kernel =
            MemoryKernel::open(temp_paths("agent-intake-id")).expect("memory kernel should open");

        let inserted = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "重新分配 id")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored");

        assert_eq!(inserted.len(), 1);
        assert_ne!(inserted[0], "agent_prop_重新分配 id");
        assert!(inserted[0].starts_with("prop_"));
        assert_eq!(kernel.proposals()[0].proposal_id, inserted[0]);
    }

    #[test]
    fn list_open_proposals_returns_only_open() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-open-list"))
            .expect("memory kernel should open");

        let inserted = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![
                    agent_proposal(MemoryType::Core, "开放提案"),
                    agent_proposal(MemoryType::Status, "将被拒绝"),
                ],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored");
        kernel
            .reject_proposal(&inserted[1], "user", Some("不需要"))
            .expect("reject should succeed");

        let open = kernel.list_open_proposals();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].proposal_id, inserted[0]);
        assert_eq!(open[0].status, MemoryProposalStatus::Open);
    }

    #[test]
    fn accept_proposal_creates_active_memory_record() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-record"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "接受后创建记忆")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        let memory_id = kernel
            .accept_proposal(&proposal_id, "user", Some("采纳"))
            .expect("accept should succeed");

        let record = kernel
            .records()
            .iter()
            .find(|record| record.memory_id == memory_id)
            .expect("accepted record should exist");
        assert_eq!(record.status, MemoryStatus::Active);
        assert_eq!(record.permanence, MemoryPermanence::Standard);
        assert_eq!(record.content, "接受后创建记忆");
    }

    #[test]
    fn accept_proposal_marks_proposal_accepted() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-status"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Working, "proposal accepted")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        kernel
            .accept_proposal(&proposal_id, "user", Some("通过"))
            .expect("accept should succeed");

        let proposal = kernel
            .proposals()
            .iter()
            .find(|proposal| proposal.proposal_id == proposal_id)
            .expect("proposal should exist");
        assert_eq!(proposal.status, MemoryProposalStatus::Accepted);
    }

    #[test]
    fn accept_proposal_writes_accepted_event() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-event"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Status, "accept event")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        let memory_id = kernel
            .accept_proposal(&proposal_id, "user", Some("通过"))
            .expect("accept should succeed");

        let event = kernel
            .events()
            .iter()
            .find(|event| {
                event.event_kind == MemoryEventKind::Accepted
                    && event.proposal_id.as_deref() == Some(proposal_id.as_str())
            })
            .expect("accepted event should exist");
        assert_eq!(event.memory_id.as_deref(), Some(memory_id.as_str()));
    }

    #[test]
    fn accept_proposal_rebuilds_projection() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-projection"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "进入 CORE projection")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        kernel
            .accept_proposal(&proposal_id, "user", Some("通过"))
            .expect("accept should succeed");

        let core = kernel
            .core_markdown()
            .expect("core projection should exist");
        assert!(core.contains("进入 CORE projection"));
        assert!(!kernel.projection_state().stale);
    }

    #[test]
    fn accept_proposal_does_not_make_memory_permanent() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-permanence"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Decision, "标准持久化")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        let memory_id = kernel
            .accept_proposal(&proposal_id, "user", Some("通过"))
            .expect("accept should succeed");

        let record = kernel
            .records()
            .iter()
            .find(|record| record.memory_id == memory_id)
            .expect("record should exist");
        assert_eq!(record.permanence, MemoryPermanence::Standard);
    }

    #[test]
    fn reject_proposal_marks_proposal_rejected() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-reject-status"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Working, "reject me")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        kernel
            .reject_proposal(&proposal_id, "user", Some("拒绝"))
            .expect("reject should succeed");

        let proposal = kernel
            .proposals()
            .iter()
            .find(|proposal| proposal.proposal_id == proposal_id)
            .expect("proposal should exist");
        assert_eq!(proposal.status, MemoryProposalStatus::Rejected);
    }

    #[test]
    fn reject_proposal_writes_rejected_event() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-reject-event"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Working, "reject event")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        kernel
            .reject_proposal(&proposal_id, "user", Some("拒绝"))
            .expect("reject should succeed");

        let event = kernel
            .events()
            .iter()
            .find(|event| {
                event.event_kind == MemoryEventKind::Rejected
                    && event.proposal_id.as_deref() == Some(proposal_id.as_str())
            })
            .expect("rejected event should exist");
        assert_eq!(event.memory_id, None);
    }

    #[test]
    fn reject_proposal_does_not_create_record() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-reject-no-record"))
            .expect("memory kernel should open");

        let proposal_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "reject no record")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();

        kernel
            .reject_proposal(&proposal_id, "user", Some("拒绝"))
            .expect("reject should succeed");

        assert!(kernel.records().is_empty());
    }

    #[test]
    fn accept_rejects_non_open_proposal() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-non-open"))
            .expect("memory kernel should open");

        let accepted_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "already accepted")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();
        kernel
            .accept_proposal(&accepted_id, "user", Some("通过"))
            .expect("accept should succeed");

        let rejected_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "already rejected")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();
        kernel
            .reject_proposal(&rejected_id, "user", Some("拒绝"))
            .expect("reject should succeed");

        let accepted_error = kernel
            .accept_proposal(&accepted_id, "user", Some("重复接受"))
            .expect_err("accepted proposal should be rejected");
        let rejected_error = kernel
            .accept_proposal(&rejected_id, "user", Some("错误接受"))
            .expect_err("rejected proposal should be rejected");

        assert!(accepted_error.0.contains("non-open proposal"));
        assert!(rejected_error.0.contains("non-open proposal"));
    }

    #[test]
    fn reject_rejects_non_open_proposal() {
        let mut kernel = MemoryKernel::open(temp_paths("proposal-reject-non-open"))
            .expect("memory kernel should open");

        let accepted_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "accepted then reject")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();
        kernel
            .accept_proposal(&accepted_id, "user", Some("通过"))
            .expect("accept should succeed");

        let rejected_id = kernel
            .submit_agent_output(MemoryAgentOutput {
                proposals: vec![agent_proposal(MemoryType::Core, "rejected then reject")],
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            })
            .expect("agent output should be stored")[0]
            .clone();
        kernel
            .reject_proposal(&rejected_id, "user", Some("拒绝"))
            .expect("reject should succeed");

        let accepted_error = kernel
            .reject_proposal(&accepted_id, "user", Some("错误拒绝"))
            .expect_err("accepted proposal should reject further reject");
        let rejected_error = kernel
            .reject_proposal(&rejected_id, "user", Some("重复拒绝"))
            .expect_err("rejected proposal should reject further reject");

        assert!(accepted_error.0.contains("non-open proposal"));
        assert!(rejected_error.0.contains("non-open proposal"));
    }
}
