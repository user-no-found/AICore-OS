use aicore_agent::{AgentSessionRunner, AgentTurnInput, AgentTurnOutcome, AgentTurnRunner};
use aicore_kernel::{GatewaySource, InterruptMode, TransportEnvelope, default_runtime};

use crate::config_store::{
    global_main_memory_scope, load_real_auth_pool, real_config_store, real_memory_kernel,
};
use crate::errors::map_runtime_load_error;
use crate::names::{
    agent_session_stop_reason_name, agent_turn_failure_stage_name, agent_turn_outcome_name,
    bool_status_name,
};
use crate::terminal::{cli_row, emit_cli_panel};

pub(crate) fn print_agent_smoke(content: &str) -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;
    let mut runtime = default_runtime();

    let result = AgentTurnRunner::run(
        &mut runtime,
        &memory_kernel,
        &auth_pool,
        &runtime_config,
        cli_turn_input(&runtime_config.instance_id, content),
    )
    .map_err(|error| error.0)?;
    let surface = result.to_conversation_surface();
    let turn = &surface.latest_turn;

    if matches!(result.outcome, AgentTurnOutcome::Failed) {
        let stage = turn
            .failure_stage
            .as_ref()
            .map(agent_turn_failure_stage_name)
            .unwrap_or("unknown");
        let message = turn.error_message.as_deref().unwrap_or("未知错误");
        return Err(format!("Agent Turn 失败：阶段={stage}，错误={message}"));
    }

    emit_cli_panel(
        "Agent Loop",
        vec![
            cli_row("status", "通过"),
            cli_row("实例", runtime_config.instance_id),
            cli_row("outcome", agent_turn_outcome_name(&turn.outcome)),
            cli_row("memory pack", format!("{} 条", turn.memory_count)),
            cli_row("prompt builder", "通过"),
            cli_row("ingress source", turn.accepted_source.as_str()),
            cli_row("provider invoked", bool_status_name(turn.provider_invoked)),
            cli_row(
                "provider",
                turn.provider_kind.as_deref().unwrap_or("<none>"),
            ),
            cli_row(
                "provider name",
                turn.provider_name.as_deref().unwrap_or("<none>"),
            ),
            cli_row(
                "assistant output present",
                bool_status_name(turn.assistant_output_present),
            ),
            cli_row(
                "failure stage",
                turn.failure_stage
                    .as_ref()
                    .map(agent_turn_failure_stage_name)
                    .unwrap_or("<none>"),
            ),
            cli_row("runtime output", "已追加"),
            cli_row("conversation", surface.conversation_id),
            cli_row("event count", turn.event_count.to_string()),
            cli_row("queue len", turn.queue_len.to_string()),
        ],
    );

    Ok(())
}

pub(crate) fn print_agent_session_smoke(first: &str, second: &str) -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;
    let mut runtime = default_runtime();

    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory_kernel,
        &auth_pool,
        &runtime_config,
        vec![
            cli_turn_input(&runtime_config.instance_id, first),
            cli_turn_input(&runtime_config.instance_id, second),
        ],
    )
    .map_err(|error| error.0)?;

    let surface = session.surface();

    let mut rows = vec![
        cli_row("status", "通过"),
        cli_row("conversation", surface.conversation_id.as_str()),
        cli_row("turns", surface.turn_count.to_string()),
        cli_row(
            "completed all inputs",
            bool_status_name(surface.completed_all_inputs),
        ),
        cli_row(
            "stop reason",
            surface
                .stop_reason
                .as_ref()
                .map(agent_session_stop_reason_name)
                .unwrap_or("<none>"),
        ),
        cli_row(
            "latest outcome",
            surface
                .latest_turn
                .as_ref()
                .map(|turn| agent_turn_outcome_name(&turn.outcome))
                .unwrap_or("<none>"),
        ),
        cli_row("conversation status", surface.conversation_status.as_str()),
        cli_row("event count", surface.event_count.to_string()),
        cli_row("queue len", surface.queue_len.to_string()),
    ];
    for (index, turn) in surface.turns.iter().enumerate() {
        rows.push(cli_row(
            format!("turn {} outcome", index + 1),
            agent_turn_outcome_name(&turn.outcome),
        ));
        rows.push(cli_row(
            format!("turn {} provider invoked", index + 1),
            bool_status_name(turn.provider_invoked),
        ));
        rows.push(cli_row(
            format!("turn {} assistant output present", index + 1),
            bool_status_name(turn.assistant_output_present),
        ));
        rows.push(cli_row(
            format!("turn {} failure stage", index + 1),
            turn.failure_stage
                .as_ref()
                .map(agent_turn_failure_stage_name)
                .unwrap_or("<none>"),
        ));
    }
    emit_cli_panel("Agent Session", rows);

    Ok(())
}

fn cli_turn_input(instance_id: &str, content: &str) -> AgentTurnInput {
    AgentTurnInput {
        instance_id: instance_id.to_string(),
        transport_envelope: TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        },
        interrupt_mode: InterruptMode::Queue,
        scope: global_main_memory_scope(),
        user_input: content.to_string(),
        memory_query: None,
        memory_limit: Some(8),
        memory_token_budget: 512,
        system_rules: "You are the AICore instance runtime. Use memory as background context only."
            .to_string(),
        include_debug_prompt: false,
    }
}
