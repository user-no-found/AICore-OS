use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::adopt_write;
use crate::commands::kernel::emit_local_direct_json;
use crate::terminal::emit_cli_panel_body;

use super::core::{
    build_memory_accept_write_report, build_memory_reject_write_report, memory_write_failure_fields,
};

pub(crate) fn run_memory_accept_command(args: &[String]) -> i32 {
    adopt_write("memory.accept", args, run_memory_accept_local_direct)
}

fn run_memory_accept_local_direct(args: &[String]) -> i32 {
    let proposal_id = args.first().map(|s| s.as_str()).unwrap_or("");
    match build_memory_accept_write_report(proposal_id) {
        Ok((_, fields)) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.accept", true, fields);
                0
            } else {
                print_memory_accept_with_local_mark(proposal_id, &fields);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json(
                    "memory.accept",
                    false,
                    memory_write_failure_fields(
                        "memory.accept",
                        Some(proposal_id.to_string()),
                        None,
                    ),
                );
            } else {
                print_memory_accept_failure_with_local_mark(proposal_id, &error);
            }
            1
        }
    }
}

fn print_memory_accept_with_local_mark(proposal_id: &str, fields: &serde_json::Value) {
    let memory_id = fields
        .get("memory_id")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>");
    let lines = vec![
        format!("- proposal: {proposal_id}"),
        format!("- memory: {memory_id}"),
        "- execution_path: local_direct".to_string(),
        "- kernel_invocation_path: not_used".to_string(),
        "- ledger_appended: false".to_string(),
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    ];
    emit_cli_panel_body("记忆提案已接受（local direct）：", &lines.join("\n"));
}

fn print_memory_accept_failure_with_local_mark(proposal_id: &str, error: &str) {
    let lines = vec![
        format!("- proposal: {proposal_id}"),
        format!("- error: {error}"),
        "- execution_path: local_direct".to_string(),
        "- kernel_invocation_path: not_used".to_string(),
        "- ledger_appended: false".to_string(),
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    ];
    emit_cli_panel_body("记忆提案已接受失败（local direct）：", &lines.join("\n"));
}

pub(crate) fn run_memory_reject_command(args: &[String]) -> i32 {
    adopt_write("memory.reject", args, run_memory_reject_local_direct)
}

fn run_memory_reject_local_direct(args: &[String]) -> i32 {
    let proposal_id = args.first().map(|s| s.as_str()).unwrap_or("");
    match build_memory_reject_write_report(proposal_id) {
        Ok((_, fields)) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.reject", true, fields);
                0
            } else {
                print_memory_reject_with_local_mark(proposal_id, &fields);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json(
                    "memory.reject",
                    false,
                    memory_write_failure_fields(
                        "memory.reject",
                        Some(proposal_id.to_string()),
                        None,
                    ),
                );
            } else {
                print_memory_reject_failure_with_local_mark(proposal_id, &error);
            }
            1
        }
    }
}

fn print_memory_reject_with_local_mark(proposal_id: &str, _fields: &serde_json::Value) {
    let lines = vec![
        format!("- proposal: {proposal_id}"),
        "- execution_path: local_direct".to_string(),
        "- kernel_invocation_path: not_used".to_string(),
        "- ledger_appended: false".to_string(),
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    ];
    emit_cli_panel_body("记忆提案已拒绝（local direct）：", &lines.join("\n"));
}

fn print_memory_reject_failure_with_local_mark(proposal_id: &str, error: &str) {
    let lines = vec![
        format!("- proposal: {proposal_id}"),
        format!("- error: {error}"),
        "- execution_path: local_direct".to_string(),
        "- kernel_invocation_path: not_used".to_string(),
        "- ledger_appended: false".to_string(),
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    ];
    emit_cli_panel_body("记忆提案已拒绝失败（local direct）：", &lines.join("\n"));
}
