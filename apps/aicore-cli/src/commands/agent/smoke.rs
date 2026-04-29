use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::extract_local_flag;
use crate::commands::kernel::{emit_local_direct_json, print_kernel_invoke_readonly};
use crate::terminal::{cli_row, emit_cli_panel};

use super::{AgentSmokeReport, build_agent_smoke_report, field};

pub(crate) fn run_agent_smoke_command(args: &[String]) -> i32 {
    let (is_local, stripped) = extract_local_flag(args);
    if stripped.is_empty() {
        eprintln!("配置命令失败：缺少 content 参数，请提供 agent smoke 内容。");
        return 1;
    }
    let content = &stripped[0];
    if is_local {
        run_agent_smoke_local_direct(content)
    } else {
        print_kernel_invoke_readonly("agent.smoke", &stripped)
    }
}

fn run_agent_smoke_local_direct(content: &str) -> i32 {
    match build_agent_smoke_report(content) {
        Ok(report) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("agent.smoke", true, report.fields());
                0
            } else {
                print_agent_smoke_with_local_mark(&report);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("agent.smoke", false, serde_json::json!({"error": error}));
            } else {
                eprintln!("配置命令失败：{error}");
            }
            1
        }
    }
}

fn print_agent_smoke_with_local_mark(report: &AgentSmokeReport) {
    let mut rows = vec![
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
    ];
    rows.push(cli_row("execution_path", "local_direct"));
    rows.push(cli_row("kernel_invocation_path", "not_used"));
    rows.push(cli_row("ledger_appended", "false"));
    rows.push(cli_row(
        "注意",
        "本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
    ));
    emit_cli_panel("Agent Loop（local direct）", rows);
}
