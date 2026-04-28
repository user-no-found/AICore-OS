use super::support::*;

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
    let mut kernel =
        MemoryKernel::open(temp_paths("search-filter-type")).expect("memory kernel should open");

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
fn fts_index_rebuilds_from_active_records() {
    let mut kernel =
        MemoryKernel::open(temp_paths("fts-rebuild")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "fts active record".to_string(),
            localized_summary: "fts active record".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    if !kernel
        .search_index_available_for_tests()
        .expect("fts capability should be readable")
    {
        return;
    }

    let results = kernel
        .search(SearchQuery {
            text: "fts active".to_string(),
            scope: None,
            memory_type: None,
            source: None,
            permanence: None,
            limit: None,
        })
        .expect("search should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].record.content, "fts active record");
}

#[test]
fn fts_index_indexes_content_normalized_and_localized_fields() {
    let mut kernel =
        MemoryKernel::open(temp_paths("fts-index-fields")).expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Working,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "field content".to_string(),
            localized_summary: "field summary".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .force_normalized_content_for_tests(&memory_id, "field normalized")
        .expect("normalized content should be mutable in tests");

    if !kernel
        .search_index_available_for_tests()
        .expect("fts capability should be readable")
    {
        return;
    }

    for query in ["content", "normalized", "summary"] {
        let results = kernel
            .search(SearchQuery {
                text: query.to_string(),
                scope: None,
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            })
            .expect("search should succeed");
        assert_eq!(results.len(), 1);
    }
}

#[test]
fn fts_search_returns_search_results() {
    let mut kernel =
        MemoryKernel::open(temp_paths("fts-search-results")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "fts result".to_string(),
            localized_summary: "fts result".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let results = kernel
        .search(SearchQuery {
            text: "fts".to_string(),
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
fn fts_search_falls_back_to_like_when_unavailable() {
    let mut kernel =
        MemoryKernel::open(temp_paths("fts-fallback")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "fallback search".to_string(),
            localized_summary: "fallback search".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .drop_search_index_for_tests()
        .expect("fts index should be droppable in tests");

    let results = kernel
        .search(SearchQuery {
            text: "fallback".to_string(),
            scope: None,
            memory_type: None,
            source: None,
            permanence: None,
            limit: None,
        })
        .expect("search should still succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].record.content, "fallback search");
}

#[test]
fn search_index_is_not_truth_source() {
    let mut kernel =
        MemoryKernel::open(temp_paths("fts-not-truth-source")).expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "search index not truth source".to_string(),
            localized_summary: "search index not truth source".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel.archive(&memory_id).expect("archive should succeed");

    let results = kernel
        .search(SearchQuery {
            text: "search index".to_string(),
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
fn search_filters_by_source() {
    let mut kernel =
        MemoryKernel::open(temp_paths("search-filter-source")).expect("memory kernel should open");

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
