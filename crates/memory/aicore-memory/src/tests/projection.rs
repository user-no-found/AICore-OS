use super::support::*;

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
    let mut kernel =
        MemoryKernel::open(temp_paths("projection-recover")).expect("memory kernel should open");
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
    let mut kernel =
        MemoryKernel::open(temp_paths("permanent-projection")).expect("memory kernel should open");

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
    let mut kernel =
        MemoryKernel::open(temp_paths("permanent-archived")).expect("memory kernel should open");

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
    let mut kernel =
        MemoryKernel::open(temp_paths("decisions-projection")).expect("memory kernel should open");

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
    let mut kernel =
        MemoryKernel::open(temp_paths("decisions-archived")).expect("memory kernel should open");

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
fn wiki_projection_creates_expected_files() {
    let paths = temp_paths("wiki-files");
    let wiki_index = paths.wiki_index_md.clone();
    let wiki_core = paths.wiki_core_md.clone();
    let wiki_decisions = paths.wiki_decisions_md.clone();
    let wiki_status = paths.wiki_status_md.clone();
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "wiki core".to_string(),
            localized_summary: "wiki core".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    assert!(wiki_index.exists());
    assert!(wiki_core.exists());
    assert!(wiki_decisions.exists());
    assert!(wiki_status.exists());
}

#[test]
fn wiki_projection_index_links_pages() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-index")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "index page".to_string(),
            localized_summary: "index page".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let index = kernel
        .wiki_index_markdown()
        .expect("wiki index should be readable");
    assert!(index.contains("[Core](core.md)"));
    assert!(index.contains("[Decisions](decisions.md)"));
    assert!(index.contains("[Status](status.md)"));
}

#[test]
fn wiki_projection_includes_active_core_records() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-core")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "核心 wiki 记录".to_string(),
            localized_summary: "核心 wiki 记录".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let wiki = kernel.wiki_core_markdown().expect("wiki core should exist");
    assert!(wiki.contains("核心 wiki 记录"));
}

#[test]
fn wiki_projection_includes_active_decision_records() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-decisions")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Decision,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "决策 wiki 记录".to_string(),
            localized_summary: "决策 wiki 记录".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let wiki = kernel
        .wiki_decisions_markdown()
        .expect("wiki decisions should exist");
    assert!(wiki.contains("决策 wiki 记录"));
}

#[test]
fn wiki_projection_includes_active_status_records() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-status")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Status,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "阶段状态记录".to_string(),
            localized_summary: "阶段状态记录".to_string(),
            state_key: Some("stage".to_string()),
            current_state: Some("P6.4.2".to_string()),
        })
        .expect("remember should succeed");

    let wiki = kernel
        .wiki_status_markdown()
        .expect("wiki status should exist");
    assert!(wiki.contains("阶段状态记录"));
}

#[test]
fn wiki_projection_excludes_archived_records() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-archived")).expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "不应进入 wiki 的归档记录".to_string(),
            localized_summary: "不应进入 wiki 的归档记录".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel.archive(&memory_id).expect("archive should succeed");

    let wiki = kernel.wiki_core_markdown().expect("wiki core should exist");
    assert!(!wiki.contains("不应进入 wiki 的归档记录"));
}

#[test]
fn wiki_projection_excludes_superseded_records() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-superseded")).expect("memory kernel should open");

    let old_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "旧 wiki 记录".to_string(),
            localized_summary: "旧 wiki 记录".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .correct_by_user(&old_id, "新 wiki 记录")
        .expect("correct should succeed");

    let wiki = kernel.wiki_core_markdown().expect("wiki core should exist");
    assert!(!wiki.contains("旧 wiki 记录"));
    assert!(wiki.contains("新 wiki 记录"));
}

#[test]
fn wiki_projection_includes_record_metadata() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-metadata")).expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Decision,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "带元数据的 wiki 记录".to_string(),
            localized_summary: "带元数据的 wiki 记录".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let wiki = kernel
        .wiki_decisions_markdown()
        .expect("wiki decisions should exist");
    assert!(wiki.contains(&format!("memory_id: {memory_id}")));
    assert!(wiki.contains("memory_type: decision"));
    assert!(wiki.contains("source: user_explicit"));
    assert!(wiki.contains("updated_at:"));
    assert!(wiki.contains("permanence: permanent"));
}

#[test]
fn wiki_projection_pages_include_freshness_metadata() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-freshness")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "freshness wiki".to_string(),
            localized_summary: "freshness wiki".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let wiki = kernel.wiki_core_markdown().expect("wiki core should exist");
    assert!(wiki.contains("generated_at:"));
    assert!(wiki.contains("last_rebuild_at:"));
    assert!(wiki.contains("projection_stale: false"));
    assert!(wiki.contains("projection_warning: <none>"));
    assert!(wiki.contains("source: memory_records"));
    assert!(wiki.contains("truth_source: memory.db / MemoryRecord / Memory Event Ledger"));
}

#[test]
fn wiki_projection_index_describes_pages() {
    let mut kernel = MemoryKernel::open(temp_paths("wiki-page-descriptions"))
        .expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "wiki index descriptions".to_string(),
            localized_summary: "wiki index descriptions".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let index = kernel
        .wiki_index_markdown()
        .expect("wiki index should be readable");
    assert!(index.contains("当前 active core 记忆列表"));
    assert!(index.contains("当前 active decision 记忆列表"));
    assert!(index.contains("当前 active status 记忆列表"));
}

#[test]
fn wiki_projection_declares_not_truth_source() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-disclaimer")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "wiki disclaimer".to_string(),
            localized_summary: "wiki disclaimer".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let index = kernel
        .wiki_index_markdown()
        .expect("wiki index should be readable");
    assert!(index.contains("这是 generated projection"));
    assert!(index.contains("不是事实来源"));
    assert!(index.contains("memory.db / MemoryRecord / Memory Event Ledger"));
    assert!(index.contains("不应手工编辑后期待反向同步"));
}

#[test]
fn wiki_projection_pages_include_record_version() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-record-version")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "wiki record version".to_string(),
            localized_summary: "wiki record version".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let wiki = kernel.wiki_core_markdown().expect("wiki core should exist");
    assert!(wiki.contains("record_version: 1"));
}

#[test]
fn wiki_projection_status_page_includes_state_metadata() {
    let mut kernel = MemoryKernel::open(temp_paths("wiki-status-state-metadata"))
        .expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Status,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "状态页元数据".to_string(),
            localized_summary: "状态页元数据".to_string(),
            state_key: Some("stage".to_string()),
            current_state: Some("P6.4.6".to_string()),
        })
        .expect("remember should succeed");

    let wiki = kernel
        .wiki_status_markdown()
        .expect("wiki status should exist");
    assert!(wiki.contains("state_key: stage"));
    assert!(wiki.contains("current_state: P6.4.6"));
}

#[test]
fn wiki_projection_atomic_write_replaces_existing_page() {
    let paths = temp_paths("wiki-atomic-write");
    let wiki_core_path = paths.wiki_core_md.clone();
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "第一次 wiki 内容".to_string(),
            localized_summary: "第一次 wiki 内容".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    fs::write(&wiki_core_path, "old stale page").expect("wiki page should be writable");
    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "第二次 wiki 内容".to_string(),
            localized_summary: "第二次 wiki 内容".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let wiki = kernel.wiki_core_markdown().expect("wiki core should exist");
    assert!(!wiki.contains("old stale page"));
    assert!(wiki.contains("第二次 wiki 内容"));
    assert!(!wiki_core_path.with_file_name("core.md.tmp").exists());
}

#[test]
fn wiki_projection_does_not_write_secret_records() {
    let mut kernel =
        MemoryKernel::open(temp_paths("wiki-secret-safety")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "api_key=topsecret".to_string(),
            localized_summary: "api_key=topsecret".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let wiki = kernel.wiki_core_markdown().expect("wiki core should exist");
    assert!(!wiki.contains("api_key=topsecret"));
}

#[test]
fn wiki_projection_failure_marks_stale_not_rollback_db() {
    let paths = temp_paths("wiki-projection-failure");
    let wiki_index = paths.wiki_index_md.clone();
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
    kernel.set_projection_failure_for_tests(true);

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "wiki 投影失败".to_string(),
            localized_summary: "wiki 投影失败".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should still succeed");

    assert_eq!(kernel.records().len(), 1);
    assert!(kernel.projection_state().stale);
    assert!(!wiki_index.exists());
}
