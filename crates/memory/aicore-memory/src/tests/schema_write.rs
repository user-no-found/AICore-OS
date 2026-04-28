use super::support::*;

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
    let mut kernel = MemoryKernel::open(temp_paths("correct")).expect("memory kernel should open");

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
            .any(|record| record.memory_id == old_id && record.status == MemoryStatus::Superseded)
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
    let mut kernel =
        MemoryKernel::open(temp_paths("correct-stale-version")).expect("memory kernel should open");

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
    let mut kernel =
        MemoryKernel::open(temp_paths("archive-stale-version")).expect("memory kernel should open");

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
    let mut kernel =
        MemoryKernel::open(temp_paths("forget-stale-version")).expect("memory kernel should open");

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
    let mut kernel =
        MemoryKernel::open(temp_paths("correct-new-version")).expect("memory kernel should open");

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
    let mut kernel =
        MemoryKernel::open(temp_paths("correct-old-version")).expect("memory kernel should open");

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
