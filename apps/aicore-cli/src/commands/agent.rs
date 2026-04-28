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

pub(crate) struct AgentSmokeReport {
    pub summary: String,
    pub fields: serde_json::Value,
}

impl AgentSmokeReport {
    pub(crate) fn summary(&self) -> String {
        self.summary.clone()
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        self.fields.clone()
    }
}

pub(crate) fn print_agent_smoke(content: &str) -> Result<(), String> {
    let report = build_agent_smoke_report(content)?;
    emit_cli_panel(
        "Agent Loop",
        vec![
            cli_row("status", "通过"),
            cli_row("实例", "global-main"),
            cli_row("outcome", field(&report.fields, "outcome")),
            cli_row(
                "memory pack",
                format!("{} 条", field(&report.fields, "memory_pack")),
            ),
            cli_row("prompt builder", "通过"),
            cli_row("ingress source", "cli"),
            cli_row(
                "provider invoked",
                field(&report.fields, "provider_invoked"),
            ),
            cli_row("provider", field(&report.fields, "provider_kind")),
            cli_row("provider name", field(&report.fields, "provider_name")),
            cli_row(
                "assistant output present",
                field(&report.fields, "assistant_output_present"),
            ),
            cli_row("failure stage", field(&report.fields, "failure_stage")),
            cli_row("runtime output", "已追加"),
            cli_row("conversation", field(&report.fields, "conversation_id")),
            cli_row("event count", field(&report.fields, "event_count")),
            cli_row("queue len", field(&report.fields, "queue_len")),
        ],
    );

    Ok(())
}

pub(crate) fn build_agent_smoke_report(content: &str) -> Result<AgentSmokeReport, String> {
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

    Ok(AgentSmokeReport {
        summary: format!(
            "Agent smoke 完成：{} / {}",
            agent_turn_outcome_name(&turn.outcome),
            surface.conversation_id
        ),
        fields: serde_json::json!({
            "operation": "agent.smoke",
            "conversation_id": surface.conversation_id,
            "outcome": agent_turn_outcome_name(&turn.outcome),
            "provider_invoked": bool_status_name(turn.provider_invoked),
            "provider_kind": turn.provider_kind.as_deref().unwrap_or("<none>"),
            "provider_name": turn.provider_name.as_deref().unwrap_or("<none>"),
            "assistant_output_present": bool_status_name(turn.assistant_output_present),
            "failure_stage": turn.failure_stage
                .as_ref()
                .map(agent_turn_failure_stage_name)
                .unwrap_or("<none>"),
            "event_count": turn.event_count.to_string(),
            "queue_len": turn.queue_len.to_string(),
            "memory_pack": turn.memory_count.to_string(),
            "kernel_invocation_path": "binary",
            "real_provider": "false",
            "tool_calling": "false",
            "streaming": "false"
        }),
    })
}

pub(crate) fn print_agent_session_smoke(first: &str, second: &str) -> Result<(), String> {
    let report = build_agent_session_smoke_report(first, second)?;
    let mut rows = vec![
        cli_row("status", "通过"),
        cli_row("conversation", field(&report.fields, "conversation_id")),
        cli_row("turns", field(&report.fields, "turn_count")),
        cli_row(
            "completed all inputs",
            bool_field_status(&report.fields, "completed_all_inputs"),
        ),
        cli_row("stop reason", field(&report.fields, "stop_reason")),
        cli_row("latest outcome", field(&report.fields, "latest_outcome")),
        cli_row(
            "conversation status",
            field(&report.fields, "conversation_status"),
        ),
        cli_row("event count", field(&report.fields, "event_count")),
        cli_row("queue len", field(&report.fields, "queue_len")),
    ];
    if let Some(turns) = report
        .fields
        .get("turns")
        .and_then(|value| value.as_array())
    {
        for (index, turn) in turns.iter().enumerate() {
            rows.push(cli_row(
                format!("turn {} outcome", index + 1),
                turn.get("outcome")
                    .and_then(|value| value.as_str())
                    .unwrap_or("<none>"),
            ));
            rows.push(cli_row(
                format!("turn {} provider invoked", index + 1),
                turn.get("provider_invoked")
                    .and_then(|value| value.as_str())
                    .unwrap_or("no"),
            ));
            rows.push(cli_row(
                format!("turn {} assistant output present", index + 1),
                turn.get("assistant_output_present")
                    .and_then(|value| value.as_str())
                    .unwrap_or("no"),
            ));
            rows.push(cli_row(
                format!("turn {} failure stage", index + 1),
                turn.get("failure_stage")
                    .and_then(|value| value.as_str())
                    .unwrap_or("<none>"),
            ));
        }
    }
    emit_cli_panel("Agent Session", rows);

    Ok(())
}

pub(crate) fn build_agent_session_smoke_report(
    first: &str,
    second: &str,
) -> Result<AgentSmokeReport, String> {
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

    let turns = surface
        .turns
        .iter()
        .map(|turn| {
            serde_json::json!({
                "outcome": agent_turn_outcome_name(&turn.outcome),
                "provider_invoked": bool_status_name(turn.provider_invoked),
                "assistant_output_present": bool_status_name(turn.assistant_output_present),
                "failure_stage": turn.failure_stage
                    .as_ref()
                    .map(agent_turn_failure_stage_name)
                    .unwrap_or("<none>")
            })
        })
        .collect::<Vec<_>>();

    Ok(AgentSmokeReport {
        summary: format!(
            "Agent session smoke 完成：{} turns / {}",
            surface.turn_count, surface.conversation_id
        ),
        fields: serde_json::json!({
            "operation": "agent.session_smoke",
            "conversation_id": surface.conversation_id,
            "turn_count": surface.turn_count.to_string(),
            "latest_outcome": surface
                .latest_turn
                .as_ref()
                .map(|turn| agent_turn_outcome_name(&turn.outcome))
                .unwrap_or("<none>"),
            "completed_all_inputs": surface.completed_all_inputs.to_string(),
            "stop_reason": surface
                .stop_reason
                .as_ref()
                .map(agent_session_stop_reason_name)
                .unwrap_or("<none>"),
            "conversation_status": surface.conversation_status,
            "event_count": surface.event_count.to_string(),
            "queue_len": surface.queue_len.to_string(),
            "turns": turns,
            "kernel_invocation_path": "binary",
            "real_provider": "false",
            "tool_calling": "false",
            "streaming": "false"
        }),
    })
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

fn field(fields: &serde_json::Value, key: &str) -> String {
    fields
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("<none>")
        .to_string()
}

fn bool_field_status(fields: &serde_json::Value, key: &str) -> &'static str {
    match fields.get(key).and_then(|value| value.as_str()) {
        Some("true") => "yes",
        Some("false") => "no",
        _ => "<none>",
    }
}
