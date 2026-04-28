use super::*;

#[test]
fn agent_session_runs_two_completed_turns() {
    let memory = MemoryKernel::open(temp_paths("agent-session-two-completed"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first request"), base_input("second request")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, 2);
    assert_eq!(session.surface().turns.len(), 2);
    assert_eq!(
        session.surface().turns[0].outcome,
        AgentTurnOutcome::Completed
    );
    assert_eq!(
        session.surface().turns[1].outcome,
        AgentTurnOutcome::Completed
    );
    assert!(session.surface().completed_all_inputs);
    assert_eq!(session.surface().stop_reason, None);
}

#[test]
fn agent_session_follow_up_turn_uses_same_conversation_id() {
    let memory = MemoryKernel::open(temp_paths("agent-session-same-conversation"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_eq!(
        session.debug_turn_outputs()[0].conversation_id,
        session.debug_turn_outputs()[1].conversation_id
    );
}

#[test]
fn agent_session_follow_up_turn_starts_after_completed_turn() {
    let memory = MemoryKernel::open(temp_paths("agent-session-follow-up-start"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_ne!(
        session.debug_turn_outputs()[0].active_turn_id,
        session.debug_turn_outputs()[1].active_turn_id
    );
    assert_eq!(runtime.turn_state().active_turn_id, None);
}

#[test]
fn agent_session_event_count_increases_across_turns() {
    let memory = MemoryKernel::open(temp_paths("agent-session-event-count"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert!(
        session.debug_turn_outputs()[1].event_count > session.debug_turn_outputs()[0].event_count
    );
}

#[test]
fn agent_session_records_turn_history_entries() {
    let memory =
        MemoryKernel::open(temp_paths("agent-session-history")).expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turns.len(), 2);
}

#[test]
fn agent_session_latest_turn_points_to_second_turn() {
    let memory = MemoryKernel::open(temp_paths("agent-session-latest-turn"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_eq!(
        session
            .surface()
            .latest_turn
            .as_ref()
            .expect("latest turn")
            .turn_id,
        session.debug_turn_outputs()[1].active_turn_id
    );
}

#[test]
fn agent_session_surface_is_public_read_contract() {
    let memory = MemoryKernel::open(temp_paths("agent-session-public-contract"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, 2);
    assert_eq!(session.surface().turns.len(), 2);
}

#[test]
fn agent_session_output_turn_outputs_are_internal_debug_result() {
    let memory = MemoryKernel::open(temp_paths("agent-session-debug-result"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_eq!(session.debug_turn_outputs().len(), 2);
    assert!(
        session.debug_turn_outputs()[0]
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .is_some()
    );
}

#[test]
fn agent_session_surface_does_not_expose_prompt() {
    let memory = MemoryKernel::open(temp_paths("agent-session-no-prompt"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    let debug_text = format!("{:?}", session.surface());
    assert!(!debug_text.contains("SYSTEM:"));
    assert!(!debug_text.contains("CURRENT USER REQUEST:"));
}

#[test]
fn agent_session_surface_does_not_expose_debug_prompt_even_when_requested() {
    let memory = MemoryKernel::open(temp_paths("agent-session-no-debug-prompt"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert!(
        session.debug_turn_outputs()[0]
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .is_some()
    );
    let public_text = format!("{:?}", session.surface());
    assert!(!public_text.contains("SYSTEM:"));
    assert!(!public_text.contains("CURRENT USER REQUEST:"));
}

#[test]
fn agent_session_surface_does_not_expose_raw_memory() {
    let paths = temp_paths("agent-session-no-raw-memory");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "session raw memory".to_string(),
            localized_summary: "session raw memory".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    let debug_text = format!("{:?}", session.surface());
    assert!(!debug_text.contains("session raw memory"));
}

#[test]
fn agent_session_default_policy_continues_all_inputs() {
    let memory = MemoryKernel::open(temp_paths("agent-session-default-policy"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        vec![base_input("failed one"), base_input("failed two")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, 2);
    assert!(session.surface().completed_all_inputs);
    assert_eq!(session.surface().stop_reason, None);
}

#[test]
fn agent_session_continue_all_records_failed_then_next_turn() {
    let memory = MemoryKernel::open(temp_paths("agent-session-continue-all"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run_with_policy(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        vec![base_input("failed one"), base_input("failed two")],
        AgentSessionContinuationPolicy::ContinueAll,
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, 2);
    assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Failed);
    assert_eq!(session.surface().turns[1].outcome, AgentTurnOutcome::Failed);
}

#[test]
fn agent_session_stop_on_failed_stops_after_failure() {
    let memory = MemoryKernel::open(temp_paths("agent-session-stop-on-failed"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run_with_policy(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        vec![base_input("failed one"), base_input("failed two")],
        AgentSessionContinuationPolicy::StopOnFailed,
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, 1);
    assert!(!session.surface().completed_all_inputs);
    assert_eq!(
        session.surface().stop_reason,
        Some(AgentSessionStopReason::Failed)
    );
}

#[test]
fn agent_session_stop_on_non_completed_stops_on_queued() {
    let memory = MemoryKernel::open(temp_paths("agent-session-stop-on-queued"))
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
    let session = AgentSessionRunner::run_with_policy(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("queued"), base_input("should not run")],
        AgentSessionContinuationPolicy::StopOnNonCompleted,
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, 1);
    assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Queued);
    assert!(!session.surface().completed_all_inputs);
    assert_eq!(
        session.surface().stop_reason,
        Some(AgentSessionStopReason::Queued)
    );
}

#[test]
fn agent_session_stop_on_non_completed_stops_on_interrupted() {
    let memory = MemoryKernel::open(temp_paths("agent-session-stop-on-interrupted"))
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
    let session = AgentSessionRunner::run_with_policy(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![
            {
                let mut input = base_input("interrupt");
                input.interrupt_mode = InterruptMode::SoftInterrupt;
                input
            },
            base_input("should not run"),
        ],
        AgentSessionContinuationPolicy::StopOnNonCompleted,
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, 1);
    assert_eq!(
        session.surface().turns[0].outcome,
        AgentTurnOutcome::Interrupted
    );
    assert!(!session.surface().completed_all_inputs);
    assert_eq!(
        session.surface().stop_reason,
        Some(AgentSessionStopReason::Interrupted)
    );
}

#[test]
fn agent_session_surface_reports_completed_all_inputs() {
    let memory = MemoryKernel::open(temp_paths("agent-session-completed-all"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert!(session.surface().completed_all_inputs);
}

#[test]
fn agent_session_surface_reports_stop_reason() {
    let memory = MemoryKernel::open(temp_paths("agent-session-stop-reason"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run_with_policy(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        vec![base_input("failed"), base_input("next")],
        AgentSessionContinuationPolicy::StopOnFailed,
    )
    .expect("session should succeed");

    assert_eq!(
        session.surface().stop_reason,
        Some(AgentSessionStopReason::Failed)
    );
}

#[test]
fn agent_session_surface_turn_count_matches_recorded_turns() {
    let memory = MemoryKernel::open(temp_paths("agent-session-turn-count-match"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turn_count, session.surface().turns.len());
}

#[test]
fn agent_session_surface_latest_turn_matches_last_recorded_turn() {
    let memory = MemoryKernel::open(temp_paths("agent-session-latest-match"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second")],
    )
    .expect("session should succeed");

    assert_eq!(
        session.surface().latest_turn,
        session.surface().turns.last().cloned()
    );
}

#[test]
fn agent_session_failed_turn_enters_history() {
    let memory = MemoryKernel::open(temp_paths("agent-session-failed-history"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_missing_auth(),
        vec![base_input("failed")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Failed);
}

#[test]
fn session_surface_records_provider_invoke_failure() {
    let memory = MemoryKernel::open(temp_paths("session-surface-provider-invoke"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_openrouter(),
        vec![base_input("provider invoke failure")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Failed);
    assert_eq!(
        session.surface().turns[0].failure_stage,
        Some(AgentTurnFailureStage::ProviderInvoke)
    );
}

#[test]
fn session_surface_provider_failure_does_not_expose_internal_request() {
    let memory = MemoryKernel::open(temp_paths("session-surface-no-internal-request"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config_openrouter(),
        vec![base_input("provider invoke failure")],
    )
    .expect("session should succeed");

    let rendered = format!("{:?}", session.surface());
    assert!(!rendered.contains("RELEVANT MEMORY:"));
    assert!(!rendered.contains("CURRENT USER REQUEST:"));
    assert!(!rendered.contains("prompt"));
}

#[test]
fn session_surface_provider_resolve_failure_does_not_expose_internal_request() {
    let memory = MemoryKernel::open(temp_paths("session-surface-resolve-no-internal-request"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool_search_only(),
        &runtime_config_search_only(),
        vec![base_input("non chat auth")],
    )
    .expect("session should succeed");

    let rendered = format!("{:?}", session.surface());
    assert!(!rendered.contains("RELEVANT MEMORY:"));
    assert!(!rendered.contains("CURRENT USER REQUEST:"));
    assert!(!rendered.contains("prompt"));
    assert!(!rendered.contains("secret://"));
}

#[test]
fn agent_session_queued_turn_enters_history() {
    let memory = MemoryKernel::open(temp_paths("agent-session-queued-history"))
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
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("queued")],
    )
    .expect("session should succeed");

    assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Queued);
}

#[test]
fn agent_session_interrupted_turn_enters_history() {
    let memory = MemoryKernel::open(temp_paths("agent-session-interrupted-history"))
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
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![{
            let mut input = base_input("interrupt");
            input.interrupt_mode = InterruptMode::SoftInterrupt;
            input
        }],
    )
    .expect("session should succeed");

    assert_eq!(
        session.surface().turns[0].outcome,
        AgentTurnOutcome::Interrupted
    );
}

#[test]
fn follow_up_turn_keeps_memory_as_background_context() {
    let paths = temp_paths("agent-session-followup-memory-background");
    let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
    memory
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "background memory item".to_string(),
            localized_summary: "background memory item".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), {
            let mut input = base_input("second");
            input.memory_query = Some("background memory".to_string());
            input
        }],
    )
    .expect("session should succeed");

    let prompt = session.debug_turn_outputs()[1]
        .debug
        .as_ref()
        .and_then(|debug| debug.prompt.as_ref())
        .expect("debug prompt should exist");
    assert!(prompt.contains("background context only"));
    assert!(prompt.contains("not the current user instruction"));
}

#[test]
fn follow_up_turn_current_user_request_still_last_in_debug_prompt() {
    let memory = MemoryKernel::open(temp_paths("agent-session-followup-request-last"))
        .expect("memory kernel should open");
    let mut runtime = default_runtime();
    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory,
        &auth_pool(),
        &runtime_config(),
        vec![base_input("first"), base_input("second request final")],
    )
    .expect("session should succeed");

    let prompt = session.debug_turn_outputs()[1]
        .debug
        .as_ref()
        .and_then(|debug| debug.prompt.as_ref())
        .expect("debug prompt should exist");
    assert!(prompt.ends_with("second request final"));
}
