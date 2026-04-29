use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::extract_local_flag;
use crate::commands::kernel::{emit_local_direct_json, print_kernel_invoke_readonly};
use crate::terminal::{cli_row, emit_cli_panel};

use super::{AgentSmokeReport, bool_field_status, build_agent_session_smoke_report, field};

pub(crate) fn run_agent_session_smoke_command(args: &[String]) -> i32 {
    let (is_local, stripped) = extract_local_flag(args);
    if stripped.len() < 2 {
        eprintln!("配置命令失败：缺少参数，agent session-smoke 需要两个输入。");
        return 1;
    }
    let first = &stripped[0];
    let second = &stripped[1];
    if is_local {
        run_agent_session_smoke_local_direct(first, second)
    } else {
        print_kernel_invoke_readonly("agent.session_smoke", &stripped)
    }
}

fn run_agent_session_smoke_local_direct(first: &str, second: &str) -> i32 {
    match build_agent_session_smoke_report(first, second) {
        Ok(report) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("agent.session_smoke", true, report.fields());
                0
            } else {
                print_agent_session_smoke_with_local_mark(&report);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json(
                    "agent.session_smoke",
                    false,
                    serde_json::json!({"error": error}),
                );
            } else {
                eprintln!("配置命令失败：{error}");
            }
            1
        }
    }
}

fn print_agent_session_smoke_with_local_mark(report: &AgentSmokeReport) {
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
    rows.push(cli_row("execution_path", "local_direct"));
    rows.push(cli_row("kernel_invocation_path", "not_used"));
    rows.push(cli_row("ledger_appended", "false"));
    rows.push(cli_row(
        "注意",
        "本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
    ));
    emit_cli_panel("Agent Session（local direct）", rows);
}
