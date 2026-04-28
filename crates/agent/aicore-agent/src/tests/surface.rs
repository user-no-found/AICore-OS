use super::*;

#[test]
fn conversation_surface_includes_completed_turn() {
    let memory = MemoryKernel::open(temp_paths("surface-completed-turn"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("completed surface"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Completed);
    assert!(surface.latest_turn.assistant_output_present);
}

#[test]
fn conversation_surface_includes_failed_turn() {
    let memory =
        MemoryKernel::open(temp_paths("surface-failed-turn")).expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("failed surface"),
    )
    .expect("agent turn should return failed outcome");

    let surface = output.to_conversation_surface();
    assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Failed);
    assert_eq!(
        surface.latest_turn.failure_stage,
        Some(AgentTurnFailureStage::ProviderResolve)
    );
}

#[test]
fn conversation_surface_includes_queued_turn() {
    let memory =
        MemoryKernel::open(temp_paths("surface-queued-turn")).expect("memory kernel should open");
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
        base_input("queued surface"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Queued);
    assert!(!surface.latest_turn.assistant_output_present);
}

#[test]
fn conversation_surface_includes_interrupted_turn() {
    let memory = MemoryKernel::open(temp_paths("surface-interrupted-turn"))
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

    let output = AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
        let mut input = base_input("interrupt surface");
        input.interrupt_mode = InterruptMode::SoftInterrupt;
        input
    })
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Interrupted);
    assert_eq!(surface.latest_turn.ingress_decision, "soft_interrupt");
}

#[test]
fn conversation_surface_includes_appended_context_turn() {
    let memory = MemoryKernel::open(temp_paths("surface-appended-context-turn"))
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

    let output = AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
        let mut input = base_input("append context");
        input.interrupt_mode = InterruptMode::AppendContext;
        input
    })
    .expect("agent turn should succeed");

    assert_eq!(output.outcome, AgentTurnOutcome::AppendedContext);
    assert!(!output.provider_invoked);
    assert_eq!(output.assistant_output, None);

    let surface = output.to_conversation_surface();
    assert_eq!(
        surface.latest_turn.outcome,
        AgentTurnOutcome::AppendedContext
    );
}

#[test]
fn conversation_surface_does_not_expose_prompt() {
    let memory =
        MemoryKernel::open(temp_paths("surface-no-prompt")).expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("no prompt surface"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    let debug_text = format!("{surface:?}");
    assert!(!debug_text.contains("SYSTEM:"));
    assert!(!debug_text.contains("CURRENT USER REQUEST:"));
}

#[test]
fn conversation_surface_does_not_expose_raw_memory() {
    let paths = temp_paths("surface-no-raw-memory");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "raw memory payload".to_string(),
            localized_summary: "raw memory payload".to_string(),
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
        base_input("raw memory"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    let debug_text = format!("{surface:?}");
    assert!(!debug_text.contains("raw memory payload"));
}

#[test]
fn conversation_surface_reports_failure_stage() {
    let memory =
        MemoryKernel::open(temp_paths("surface-failure-stage")).expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("failure stage"),
    )
    .expect("agent turn should return failed outcome");

    let surface = output.to_conversation_surface();
    assert_eq!(
        surface.latest_turn.failure_stage,
        Some(AgentTurnFailureStage::ProviderResolve)
    );
}

#[test]
fn conversation_surface_reports_provider_invoke_failure() {
    let memory = MemoryKernel::open(temp_paths("conversation-surface-provider-invoke"))
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
    let surface = output.to_conversation_surface();

    assert_eq!(
        surface.latest_turn.failure_stage,
        Some(AgentTurnFailureStage::ProviderInvoke)
    );
    assert_eq!(
        surface.latest_turn.provider_kind.as_deref(),
        Some("openrouter")
    );
}

#[test]
fn conversation_surface_reports_provider_metadata_when_invoked() {
    let memory = MemoryKernel::open(temp_paths("surface-provider-metadata"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("provider metadata"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    assert!(surface.latest_turn.provider_invoked);
    assert_eq!(surface.latest_turn.provider_kind.as_deref(), Some("dummy"));
    assert_eq!(surface.latest_turn.provider_name.as_deref(), Some("dummy"));
}

#[test]
fn conversation_surface_reports_no_provider_when_not_invoked() {
    let memory =
        MemoryKernel::open(temp_paths("surface-no-provider")).expect("memory kernel should open");
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
        base_input("queued surface"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    assert!(!surface.latest_turn.provider_invoked);
    assert_eq!(surface.latest_turn.provider_kind, None);
    assert_eq!(surface.latest_turn.provider_name, None);
}

#[test]
fn agent_turn_provider_failure_surface_does_not_expose_secret_ref() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-no-secret-ref"))
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

    let surface = output.to_conversation_surface();
    let rendered = format!("{surface:?}");
    assert!(!rendered.contains("secret://"));
}

#[test]
fn agent_turn_provider_resolve_failure_surface_does_not_expose_secret_ref() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-resolve-no-secret-ref"))
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

    let surface = output.to_conversation_surface();
    let rendered = format!("{surface:?}");
    assert!(!rendered.contains("secret://"));
}

#[test]
fn conversation_surface_preserves_conversation_id() {
    let memory = MemoryKernel::open(temp_paths("surface-conversation-id"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("conversation id"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    assert_eq!(surface.conversation_id, output.conversation_id);
    assert_eq!(surface.latest_turn.conversation_id, output.conversation_id);
}

#[test]
fn conversation_surface_uses_runtime_event_count() {
    let memory =
        MemoryKernel::open(temp_paths("surface-event-count")).expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("event count"),
    )
    .expect("agent turn should succeed");

    let surface = output.to_conversation_surface();
    assert_eq!(surface.latest_turn.event_count, output.event_count);
}

#[test]
fn agent_turn_completed_can_be_converted_to_surface_entry() {
    let memory = MemoryKernel::open(temp_paths("surface-entry-completed"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("completed entry"),
    )
    .expect("agent turn should succeed");

    let entry = output.to_surface_entry();
    assert_eq!(entry.outcome, AgentTurnOutcome::Completed);
}

#[test]
fn agent_turn_failed_can_be_converted_to_surface_entry() {
    let memory =
        MemoryKernel::open(temp_paths("surface-entry-failed")).expect("memory kernel should open");
    let mut runtime = default_runtime();

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        base_input("failed entry"),
    )
    .expect("agent turn should return failed outcome");

    let entry = output.to_surface_entry();
    assert_eq!(entry.outcome, AgentTurnOutcome::Failed);
}

#[test]
fn agent_turn_queued_can_be_converted_to_surface_entry() {
    let memory =
        MemoryKernel::open(temp_paths("surface-entry-queued")).expect("memory kernel should open");
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
        base_input("queued entry"),
    )
    .expect("agent turn should succeed");

    let entry = output.to_surface_entry();
    assert_eq!(entry.outcome, AgentTurnOutcome::Queued);
}

#[test]
fn agent_turn_interrupted_can_be_converted_to_surface_entry() {
    let memory = MemoryKernel::open(temp_paths("surface-entry-interrupted"))
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

    let output = AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
        let mut input = base_input("interrupted entry");
        input.interrupt_mode = InterruptMode::SoftInterrupt;
        input
    })
    .expect("agent turn should succeed");

    let entry = output.to_surface_entry();
    assert_eq!(entry.outcome, AgentTurnOutcome::Interrupted);
}

#[test]
fn agent_turn_appended_context_can_be_converted_to_surface_entry() {
    let memory = MemoryKernel::open(temp_paths("surface-entry-appended-context"))
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

    let output = AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
        let mut input = base_input("append context entry");
        input.interrupt_mode = InterruptMode::AppendContext;
        input
    })
    .expect("agent turn should succeed");

    let entry = output.to_surface_entry();
    assert_eq!(entry.outcome, AgentTurnOutcome::AppendedContext);
}

#[test]
fn agent_turn_does_not_auto_accept_memory_proposals() {
    let paths = temp_paths("agent-turn-no-auto-accept");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    memory
        .submit_assistant_summary(global_scope(), "proposal only memory")
        .expect("proposal should be created");
    let proposal_count = memory.proposals().len();
    let record_count = memory.records().len();

    let mut runtime = default_runtime();
    let _ = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("proposal should stay open"),
    )
    .expect("agent turn should succeed");

    assert_eq!(memory.proposals().len(), proposal_count);
    assert_eq!(memory.records().len(), record_count);
}
