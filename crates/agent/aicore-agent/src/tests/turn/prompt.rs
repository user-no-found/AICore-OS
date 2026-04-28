use super::super::*;

#[test]
fn agent_turn_builds_prompt_with_memory_context() {
    let paths = temp_paths("agent-turn-memory-context");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "memory context item".to_string(),
            localized_summary: "memory context item".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let mut runtime = default_runtime();
    let mut input = base_input("use memory context");
    input.memory_query = Some("memory context".to_string());
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        input,
    )
    .expect("agent turn should succeed");

    let prompt = output
        .debug
        .as_ref()
        .and_then(|debug| debug.prompt.as_ref())
        .expect("debug prompt should exist");
    assert!(prompt.contains("RELEVANT MEMORY:"));
    assert!(prompt.contains("memory context item"));
}

#[test]
fn agent_turn_marks_memory_as_background_context() {
    let memory =
        MemoryKernel::open(temp_paths("agent-turn-background")).expect("memory kernel should open");
    let mut runtime = default_runtime();
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("do work"),
    )
    .expect("agent turn should succeed");

    let prompt = output
        .debug
        .as_ref()
        .and_then(|debug| debug.prompt.as_ref())
        .expect("debug prompt should exist");
    assert!(prompt.contains("background context only"));
    assert!(prompt.contains("not the current user instruction"));
}

#[test]
fn agent_turn_puts_current_user_request_last() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-request-last"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("final request section"),
    )
    .expect("agent turn should succeed");

    let prompt = output
        .debug
        .as_ref()
        .and_then(|debug| debug.prompt.as_ref())
        .expect("debug prompt should exist");
    assert!(prompt.ends_with("final request section"));
}

#[test]
fn agent_turn_uses_search_result_order_for_memory_pack() {
    let paths = temp_paths("agent-turn-search-order");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Decision,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "shared memory".to_string(),
            localized_summary: "shared memory".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "shared memory".to_string(),
            localized_summary: "shared memory".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let mut runtime = default_runtime();
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("shared"),
    )
    .expect("agent turn should succeed");

    let prompt = output
        .debug
        .as_ref()
        .and_then(|debug| debug.prompt.as_ref())
        .expect("debug prompt should exist");
    let core_pos = prompt.find("[core]").expect("core memory should exist");
    let decision_pos = prompt
        .find("[decision]")
        .expect("decision memory should exist");
    assert!(core_pos < decision_pos);
}

#[test]
fn agent_turn_excludes_archived_memory() {
    let paths = temp_paths("agent-turn-archived");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    let archived_id = memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "archived context".to_string(),
            localized_summary: "archived context".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    memory
        .archive(&archived_id)
        .expect("archive should succeed");

    let mut runtime = default_runtime();
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("archived"),
    )
    .expect("agent turn should succeed");

    let prompt = output
        .debug
        .as_ref()
        .and_then(|debug| debug.prompt.as_ref())
        .expect("debug prompt should exist");
    assert!(!prompt.contains("archived context"));
    assert_eq!(output.memory_count, 0);
}
