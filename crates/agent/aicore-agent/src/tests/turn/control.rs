use super::super::*;

#[test]
fn agent_turn_queue_decision_does_not_invoke_provider() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-queue-no-provider"))
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

    assert_eq!(output.outcome, AgentTurnOutcome::Queued);
    assert!(!output.provider_invoked);
    assert!(!output.assistant_output_generated);
    assert_eq!(output.assistant_output, None);
}

#[test]
fn agent_turn_queue_decision_does_not_append_assistant_output() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-queue-no-append"))
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
    let initial_event_count = runtime.summary().event_count;

    let output = AgentTurnRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        base_input("queued input"),
    )
    .expect("agent turn should succeed");

    assert_eq!(output.event_count, runtime.summary().event_count);
    assert_eq!(runtime.summary().event_count, initial_event_count);
}

#[test]
fn agent_turn_queue_result_reports_queue_len() {
    let memory =
        MemoryKernel::open(temp_paths("agent-turn-queue-len")).expect("memory kernel should open");
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

    assert_eq!(output.outcome, AgentTurnOutcome::Queued);
    assert!(output.queue_len >= 1);
}

#[test]
fn agent_turn_soft_interrupt_does_not_invoke_provider() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-soft-interrupt"))
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
        let mut input = base_input("soft interrupt");
        input.interrupt_mode = InterruptMode::SoftInterrupt;
        input
    })
    .expect("agent turn should succeed");

    assert_eq!(output.outcome, AgentTurnOutcome::Interrupted);
    assert_eq!(output.ingress_decision, "soft_interrupt");
    assert!(!output.provider_invoked);
}

#[test]
fn agent_turn_hard_interrupt_does_not_invoke_provider() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-hard-interrupt"))
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
        let mut input = base_input("hard interrupt");
        input.interrupt_mode = InterruptMode::HardInterrupt;
        input
    })
    .expect("agent turn should succeed");

    assert_eq!(output.outcome, AgentTurnOutcome::Interrupted);
    assert_eq!(output.ingress_decision, "hard_interrupt");
    assert!(!output.provider_invoked);
}

#[test]
fn agent_turn_interrupt_result_reports_decision() {
    let memory = MemoryKernel::open(temp_paths("agent-turn-interrupt-decision"))
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
        let mut input = base_input("soft interrupt");
        input.interrupt_mode = InterruptMode::SoftInterrupt;
        input
    })
    .expect("agent turn should succeed");

    assert_eq!(output.ingress_decision, "soft_interrupt");
    assert_eq!(output.active_turn_status.as_deref(), Some("interrupted"));
    assert_eq!(output.conversation_status, "interrupted");
}
