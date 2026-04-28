use super::super::*;

#[test]
fn agent_turn_uses_provider_resolver_and_dummy_provider() {
    let memory =
        MemoryKernel::open(temp_paths("agent-turn-provider")).expect("memory kernel should open");
    let mut runtime = default_runtime();
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("provider test"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.provider_name.as_deref(), Some("dummy"));
    assert_eq!(output.provider_kind.as_deref(), Some("dummy"));
    assert!(
        output
            .assistant_output
            .as_deref()
            .expect("assistant output should exist")
            .contains("dummy provider response")
    );
}

#[test]
fn agent_turn_appends_assistant_output_to_runtime() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-runtime-append"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("append output"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.outcome, AgentTurnOutcome::Completed);
    assert!(output.runtime_output_ok);
    assert!(output.event_count >= 2);
    assert!(output.active_turn_id.is_some());
    assert_eq!(output.conversation_status, "idle");
    assert_eq!(output.active_turn_status, None);
}

#[test]
fn agent_turn_returns_memory_count() {
    let paths = temp_paths("agent-turn-memory-count");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "counted memory".to_string(),
            localized_summary: "counted memory".to_string(),
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
        base_input("counted"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.memory_count, 1);
    assert!(output.prompt_builder_ok);
}

#[test]
fn agent_turn_public_output_does_not_expose_full_prompt() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-public-no-prompt"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let mut input = base_input("hide prompt");
    input.include_debug_prompt = false;

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        input,
    )
    .expect("agent turn should succeed");

    let debug = output.debug.expect("debug metadata should exist");
    assert!(debug.prompt.is_none());
    assert!(debug.prompt_length > 0);
}

#[test]
fn agent_turn_debug_prompt_requires_explicit_request() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-debug-explicit"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let without_debug =
        AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
            let mut input = base_input("no debug prompt");
            input.include_debug_prompt = false;
            input
        })
        .expect("agent turn should succeed");
    assert!(
        without_debug
            .debug
            .as_ref()
            .expect("debug metadata should exist")
            .prompt
            .is_none()
    );

    let with_debug =
        AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
            let mut input = base_input("with debug prompt");
            input.include_debug_prompt = true;
            input
        })
        .expect("agent turn should succeed");
    assert!(
        with_debug
            .debug
            .as_ref()
            .expect("debug metadata should exist")
            .prompt
            .is_some()
    );
}

#[test]
fn agent_turn_returns_conversation_surface_metadata() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-surface-metadata"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("surface metadata"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.accepted_source, "cli");
    assert!(!output.ingress_decision.is_empty());
    assert!(!output.conversation_id.is_empty());
    assert!(output.event_count >= 2);
}

#[test]
fn agent_turn_uses_supplied_transport_envelope() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-supplied-envelope"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let mut input = base_input("external envelope");
    input.transport_envelope = TransportEnvelope {
        source: GatewaySource::External,
        platform: Some("feishu".to_string()),
        target_id: Some("chat-1".to_string()),
        sender_id: Some("user-1".to_string()),
        is_group: true,
        mentioned_bot: true,
    };

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        input,
    )
    .expect("agent turn should succeed");

    assert_eq!(output.accepted_source, "external");
}

#[test]
fn agent_turn_no_longer_hardcodes_cli_envelope() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-no-hardcoded-cli"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let mut input = base_input("tui envelope");
    input.transport_envelope = TransportEnvelope {
        source: GatewaySource::Tui,
        platform: None,
        target_id: None,
        sender_id: None,
        is_group: false,
        mentioned_bot: false,
    };

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        input,
    )
    .expect("agent turn should succeed");

    assert_eq!(output.accepted_source, "tui");
}

#[test]
fn agent_turn_start_turn_invokes_provider_and_appends_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-start-turn-provider"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("start turn"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.outcome, AgentTurnOutcome::Completed);
    assert!(output.provider_invoked);
    assert!(output.assistant_output_generated);
    assert!(output.assistant_output.is_some());
}

#[test]
fn agent_turn_start_turn_completes_turn() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-completes-turn"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("complete turn"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.outcome, AgentTurnOutcome::Completed);
    assert_eq!(runtime.turn_state().active_turn_id, None);
    assert_eq!(
        runtime.summary().conversation_status,
        aicore_kernel::ConversationStatus::Idle
    );
}
