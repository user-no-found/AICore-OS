use super::*;

#[test]
fn prompt_builder_includes_background_memory() {
    let prompt = PromptBuilder::build(PromptBuildInput {
        instance_id: "global-main".to_string(),
        system_rules: "You are the AICore instance runtime.".to_string(),
        relevant_memory: vec![],
        user_request: "请总结当前状态".to_string(),
    })
    .prompt;

    assert!(prompt.contains("MEMORY SNAPSHOT:"));
    assert!(prompt.contains("background context only"));
}

#[test]
fn prompt_builder_marks_memory_as_not_current_instruction() {
    let prompt = PromptBuilder::build(PromptBuildInput {
        instance_id: "global-main".to_string(),
        system_rules: "You are the AICore instance runtime.".to_string(),
        relevant_memory: vec![],
        user_request: "继续实现".to_string(),
    })
    .prompt;

    assert!(prompt.contains("not the current user instruction"));
    assert!(prompt.contains("not as the latest request"));
}

#[test]
fn prompt_builder_puts_current_user_request_last() {
    let prompt = PromptBuilder::build(PromptBuildInput {
        instance_id: "global-main".to_string(),
        system_rules: "You are the AICore instance runtime.".to_string(),
        relevant_memory: vec![],
        user_request: "最后的用户请求".to_string(),
    })
    .prompt;

    assert!(prompt.ends_with("最后的用户请求"));
    let current_request_pos = prompt.find("CURRENT USER REQUEST:").unwrap();
    let memory_pos = prompt.find("RELEVANT MEMORY:").unwrap();
    assert!(current_request_pos > memory_pos);
}

#[test]
fn prompt_builder_respects_memory_pack_limit() {
    let paths = temp_paths("prompt-pack-limit");
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "重要长期记忆".to_string(),
            localized_summary: "重要长期记忆".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Working,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "第二条较长记忆内容".to_string(),
            localized_summary: "第二条较长记忆内容".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let pack = kernel.build_memory_context_pack(
        SearchQuery {
            text: String::new(),
            scope: Some(global_scope()),
            memory_type: None,
            source: None,
            permanence: None,
            limit: None,
        },
        8,
    );

    assert_eq!(pack.len(), 1);
    assert_eq!(pack[0].localized_summary, "重要长期记忆");
}

#[test]
fn prompt_builder_excludes_archived_memory() {
    let paths = temp_paths("prompt-pack-archived");
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

    let active_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "active memory".to_string(),
            localized_summary: "active memory".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    let archived_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "archived memory".to_string(),
            localized_summary: "archived memory".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .archive(&archived_id)
        .expect("archive should succeed");

    let pack = kernel.build_memory_context_pack(
        SearchQuery {
            text: "memory".to_string(),
            scope: Some(global_scope()),
            memory_type: None,
            source: None,
            permanence: None,
            limit: None,
        },
        128,
    );

    assert_eq!(pack.len(), 1);
    assert_eq!(pack[0].memory_id, active_id);
}

#[test]
fn prompt_builder_uses_search_result_order() {
    let paths = temp_paths("prompt-pack-order");
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

    let decision_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Decision,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "shared request".to_string(),
            localized_summary: "shared request".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    let core_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "shared request".to_string(),
            localized_summary: "shared request".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let pack = kernel.build_memory_context_pack(
        SearchQuery {
            text: "shared".to_string(),
            scope: Some(global_scope()),
            memory_type: None,
            source: None,
            permanence: None,
            limit: None,
        },
        128,
    );

    assert_eq!(pack.len(), 2);
    assert_eq!(pack[0].memory_id, core_id);
    assert_eq!(pack[1].memory_id, decision_id);
}
