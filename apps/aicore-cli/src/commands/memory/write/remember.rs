use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::adopt_write;
use crate::commands::kernel::emit_local_direct_json;
use crate::terminal::emit_cli_panel_body;

use super::core::{build_memory_remember_write_report, memory_write_failure_fields};

pub(crate) fn run_memory_remember_command(args: &[String]) -> i32 {
    adopt_write("memory.remember", args, run_memory_remember_local_direct)
}

fn run_memory_remember_local_direct(args: &[String]) -> i32 {
    let content = args.first().map(|s| s.as_str()).unwrap_or("");
    match build_memory_remember_write_report(content) {
        Ok((_, mut fields)) => {
            // Local direct path: override kernel_invocation_path to not_used
            if let Some(obj) = fields.as_object_mut() {
                obj.insert(
                    "kernel_invocation_path".to_string(),
                    serde_json::Value::String("not_used".to_string()),
                );
            }
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.remember", true, fields);
                0
            } else {
                print_memory_remember_with_local_mark(content, &fields);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json(
                    "memory.remember",
                    false,
                    memory_write_failure_fields(
                        "memory.remember",
                        None,
                        Some(content.chars().count()),
                    ),
                );
            } else {
                print_memory_remember_failure_with_local_mark(content, &error);
            }
            1
        }
    }
}

fn print_memory_remember_with_local_mark(content: &str, fields: &serde_json::Value) {
    let memory_id = fields
        .get("memory_id")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>");
    let preview = if content.chars().count() > 50 {
        format!(
            "{}...",
            &content[..content
                .char_indices()
                .nth(50)
                .map(|(i, _)| i)
                .unwrap_or(content.len())]
        )
    } else {
        content.to_string()
    };
    let lines = vec![
        format!("- content: {preview}"),
        format!("- id: {memory_id}"),
        "- type: core".to_string(),
        "- execution_path: local_direct".to_string(),
        "- kernel_invocation_path: not_used".to_string(),
        "- ledger_appended: false".to_string(),
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    ];
    emit_cli_panel_body("记忆已写入（local direct）：", &lines.join("\n"));
}

fn print_memory_remember_failure_with_local_mark(content: &str, error: &str) {
    let preview = if content.chars().count() > 50 {
        format!(
            "{}...",
            &content[..content
                .char_indices()
                .nth(50)
                .map(|(i, _)| i)
                .unwrap_or(content.len())]
        )
    } else {
        content.to_string()
    };
    let lines = vec![
        format!("- content: {preview}"),
        format!("- error: {error}"),
        "- execution_path: local_direct".to_string(),
        "- kernel_invocation_path: not_used".to_string(),
        "- ledger_appended: false".to_string(),
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    ];
    emit_cli_panel_body("记忆写入失败（local direct）：", &lines.join("\n"));
}
