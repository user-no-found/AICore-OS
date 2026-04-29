use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::extract_local_flag;
use crate::commands::kernel::{emit_local_direct_json, print_kernel_invoke_readonly};
use crate::commands::memory::report::{
    build_memory_audit_report, build_memory_proposals_report, build_memory_status_report,
};
use crate::terminal::emit_cli_panel_body;

pub(crate) fn run_memory_status_command(args: &[String]) -> i32 {
    let (is_local, stripped) = extract_local_flag(args);
    if is_local {
        run_memory_status_local_direct()
    } else {
        print_kernel_invoke_readonly("memory.status", &stripped)
    }
}

fn run_memory_status_local_direct() -> i32 {
    match build_memory_status_report() {
        Ok((_, fields)) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.status", true, fields);
                0
            } else {
                print_memory_status_with_local_mark(&fields);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.status", false, serde_json::json!({"error": error}));
            } else {
                eprintln!("记忆命令失败：{error}");
            }
            1
        }
    }
}

fn print_memory_status_with_local_mark(fields: &serde_json::Value) {
    let mut lines = vec![
        format!("- instance: {}", field_str(fields, "scope")),
        format!("- root: {}", field_str(fields, "memory_root")),
        format!("- records: {}", field_str(fields, "record_count")),
        format!("- proposals: {}", field_str(fields, "proposal_count")),
        format!("- events: {}", field_str(fields, "event_count")),
        format!(
            "- projection stale: {}",
            field_str(fields, "projection_stale")
        ),
        format!(
            "- projection warning: {}",
            field_str(fields, "projection_warning")
        ),
        format!(
            "- last rebuild at: {}",
            field_str(fields, "last_rebuild_at")
        ),
    ];
    lines.push("- execution_path: local_direct".to_string());
    lines.push("- kernel_invocation_path: not_used".to_string());
    lines.push("- ledger_appended: false".to_string());
    lines.push(
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    );
    emit_cli_panel_body("Memory Status（local direct）：", &lines.join("\n"));
}

pub(crate) fn run_memory_audit_command(args: &[String]) -> i32 {
    let (is_local, stripped) = extract_local_flag(args);
    if is_local {
        run_memory_audit_local_direct()
    } else {
        print_kernel_invoke_readonly("memory.audit", &stripped)
    }
}

fn run_memory_audit_local_direct() -> i32 {
    match build_memory_audit_report() {
        Ok((_, fields)) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.audit", true, fields);
                0
            } else {
                print_memory_audit_with_local_mark(&fields);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.audit", false, serde_json::json!({"error": error}));
            } else {
                eprintln!("记忆命令失败：{error}");
            }
            1
        }
    }
}

fn print_memory_audit_with_local_mark(fields: &serde_json::Value) {
    let mut lines = vec![
        format!("- checked events: {}", field_str(fields, "checked_events")),
        format!(
            "- status: {}",
            if field_str(fields, "ok") == "true" {
                "ok"
            } else {
                "failed"
            }
        ),
    ];
    if let Some(errors) = fields.get("errors").and_then(|v| v.as_str()) {
        if let Ok(issues) = serde_json::from_str::<Vec<String>>(errors) {
            for issue in issues {
                lines.push(format!("- issue: {issue}"));
            }
        }
    }
    lines.push("- execution_path: local_direct".to_string());
    lines.push("- kernel_invocation_path: not_used".to_string());
    lines.push("- ledger_appended: false".to_string());
    lines.push(
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    );
    emit_cli_panel_body("Memory Audit（local direct）：", &lines.join("\n"));
}

pub(crate) fn run_memory_proposals_command(args: &[String]) -> i32 {
    let (is_local, stripped) = extract_local_flag(args);
    if is_local {
        run_memory_proposals_local_direct()
    } else {
        print_kernel_invoke_readonly("memory.proposals", &stripped)
    }
}

fn run_memory_proposals_local_direct() -> i32 {
    match build_memory_proposals_report() {
        Ok((_, fields)) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.proposals", true, fields);
                0
            } else {
                print_memory_proposals_with_local_mark(&fields);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json(
                    "memory.proposals",
                    false,
                    serde_json::json!({"error": error}),
                );
            } else {
                eprintln!("记忆命令失败：{error}");
            }
            1
        }
    }
}

fn print_memory_proposals_with_local_mark(fields: &serde_json::Value) {
    let mut body = String::new();
    if let Some(proposals_str) = fields.get("proposals").and_then(|v| v.as_str()) {
        if let Ok(proposals) = serde_json::from_str::<Vec<serde_json::Value>>(proposals_str) {
            if proposals.is_empty() {
                body.push_str("暂无待审阅记忆提案。");
            } else {
                let lines: Vec<String> = proposals
                    .iter()
                    .map(|p| {
                        format!(
                            "- {} [{}] {}",
                            field_str(p, "proposal_id"),
                            field_str(p, "memory_type"),
                            field_str(p, "content")
                        )
                    })
                    .collect();
                body.push_str(&lines.join("\n"));
            }
        } else {
            body.push_str("暂无待审阅记忆提案。");
        }
    } else {
        body.push_str("暂无待审阅记忆提案。");
    }
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str("- execution_path: local_direct\n");
    body.push_str("- kernel_invocation_path: not_used\n");
    body.push_str("- ledger_appended: false\n");
    body.push_str(
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
    );
    emit_cli_panel_body("Memory Proposals（local direct）：", &body);
}

fn field_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("<none>")
        .to_string()
}
