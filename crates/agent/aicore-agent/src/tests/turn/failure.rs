use super::super::*;

#[test]
fn agent_turn_completed_result_contains_assistant_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-completed-output"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("completed output"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.outcome, AgentTurnOutcome::Completed);
    assert!(output.assistant_output.is_some());
}

#[test]
fn agent_turn_non_generated_result_has_no_assistant_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-no-generated-output"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        },
        "existing turn",
        InterruptMode::Queue,
    );

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("queued input"),
    )
    .expect("agent turn should succeed");

    assert_ne!(output.outcome, AgentTurnOutcome::Completed);
    assert_eq!(output.assistant_output, None);
}

#[test]
fn agent_turn_provider_resolve_failure_returns_failed_outcome() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-failed"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("provider resolve failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.outcome, AgentTurnOutcome::Failed);
    assert_eq!(
        output.failure_stage,
        Some(AgentTurnFailureStage::ProviderResolve)
    );
}

#[test]
fn agent_turn_provider_resolve_failure_does_not_invoke_provider() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-no-provider"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("provider resolve failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.provider_invoked, false);
    assert_eq!(output.assistant_output_generated, false);
}

#[test]
fn agent_turn_non_chat_auth_returns_provider_resolve_failure() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-non-chat-auth-failed"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool_search_only(),
        &runtime_config_search_only(),
        base_input("non chat auth"),
    )
    .expect("agent turn should return structured resolve failure");

    assert_eq!(output.outcome, AgentTurnOutcome::Failed);
    assert_eq!(
        output.failure_stage,
        Some(AgentTurnFailureStage::ProviderResolve)
    );
    assert_eq!(output.provider_invoked, false);
}

#[test]
fn agent_turn_non_chat_auth_does_not_append_assistant_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-non-chat-auth-no-append"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let initial_event_count = runtime.summary().event_count;

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool_search_only(),
        &runtime_config_search_only(),
        base_input("non chat auth"),
    )
    .expect("agent turn should return structured resolve failure");

    assert_eq!(output.event_count, initial_event_count + 1);
    assert_eq!(runtime.summary().event_count, initial_event_count + 1);
    assert_eq!(output.assistant_output, None);
}

#[test]
fn agent_turn_non_chat_auth_does_not_leave_turn_running() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-non-chat-auth-no-running"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool_search_only(),
        &runtime_config_search_only(),
        base_input("non chat auth"),
    )
    .expect("agent turn should return structured resolve failure");

    assert_eq!(output.outcome, AgentTurnOutcome::Failed);
    assert_eq!(runtime.turn_state().active_turn_id, None);
    assert_eq!(
        runtime.summary().conversation_status,
        aicore_kernel::ConversationStatus::Idle
    );
}

#[test]
fn agent_turn_real_provider_unavailable_returns_failed_outcome() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-failed"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_openrouter(),
        base_input("provider invoke failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.outcome, AgentTurnOutcome::Failed);
    assert_eq!(
        output.failure_stage,
        Some(AgentTurnFailureStage::ProviderInvoke)
    );
    assert_eq!(output.provider_invoked, false);
    assert_eq!(output.assistant_output, None);
}

#[test]
fn agent_turn_provider_invoke_failure_does_not_append_assistant_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-no-append"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let initial_event_count = runtime.summary().event_count;

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_openrouter(),
        base_input("provider invoke failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.event_count, initial_event_count + 1);
    assert_eq!(runtime.summary().event_count, initial_event_count + 1);
    assert_eq!(output.assistant_output, None);
}

#[test]
fn agent_turn_provider_invoke_failure_does_not_leave_turn_running() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-no-running"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_openrouter(),
        base_input("provider invoke failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.outcome, AgentTurnOutcome::Failed);
    assert_eq!(runtime.turn_state().active_turn_id, None);
    assert_eq!(
        runtime.summary().conversation_status,
        aicore_kernel::ConversationStatus::Idle
    );
}

#[test]
fn agent_turn_provider_invoke_failure_reports_failure_stage() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-stage"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_openrouter(),
        base_input("provider invoke failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(
        output.failure_stage,
        Some(AgentTurnFailureStage::ProviderInvoke)
    );
    assert_eq!(output.provider_kind.as_deref(), Some("openrouter"));
    assert_eq!(output.provider_name.as_deref(), Some("openrouter"));
    assert!(
        output
            .error_message
            .as_deref()
            .expect("failure message should exist")
            .contains("Provider")
    );
    assert!(
        !output
            .error_message
            .as_deref()
            .expect("failure message should exist")
            .contains("secret://")
    );
}

#[test]
fn agent_turn_provider_resolve_failure_does_not_append_assistant_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-no-append"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let initial_event_count = runtime.summary().event_count;

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("provider resolve failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.event_count, initial_event_count + 1);
    assert_eq!(runtime.summary().event_count, initial_event_count + 1);
}

#[test]
fn agent_turn_provider_resolve_failure_does_not_leave_active_turn_running() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-no-running-turn"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("provider resolve failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.outcome, AgentTurnOutcome::Failed);
    assert_eq!(runtime.turn_state().active_turn_id, None);
    assert_eq!(
        runtime.summary().conversation_status,
        aicore_kernel::ConversationStatus::Idle
    );
}

#[test]
fn agent_turn_failed_result_has_no_assistant_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-failed-no-output"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("provider resolve failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(output.outcome, AgentTurnOutcome::Failed);
    assert_eq!(output.assistant_output, None);
}

#[test]
fn agent_turn_failed_result_reports_failure_stage() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-failure-stage"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("provider resolve failure"),
    )
    .expect("agent turn should return structured failure");

    assert_eq!(
        output.failure_stage,
        Some(AgentTurnFailureStage::ProviderResolve)
    );
    assert!(
        output
            .error_message
            .as_deref()
            .expect("failure message should exist")
            .contains("auth")
    );
}

#[test]
fn agent_turn_non_generated_debug_prompt_is_none() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-non-generated-no-debug-prompt"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        },
        "existing turn",
        InterruptMode::Queue,
    );

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("queued input"),
    )
    .expect("agent turn should succeed");

    assert!(
        output
            .debug
            .as_ref()
            .expect("debug metadata should exist")
            .prompt
            .is_none()
    );
}
