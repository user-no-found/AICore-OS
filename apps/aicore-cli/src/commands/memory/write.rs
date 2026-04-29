use aicore_memory::{MemoryPermanence, MemoryType, RememberInput};
use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::adopt_write;
use crate::commands::kernel::emit_local_direct_json;
use crate::config_store::{global_main_memory_scope, real_memory_kernel};
use crate::errors::memory_error;
use crate::terminal::emit_cli_panel_body;

pub(crate) fn print_memory_remember(content: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_main_memory_scope(),
            content: content.to_string(),
            localized_summary: content.to_string(),
            state_key: None,
            current_state: None,
        })
        .map_err(memory_error)?;

    emit_cli_panel_body(
        "记忆已写入：",
        &[
            format!("- id: {memory_id}"),
            "- type: core".to_string(),
            "- status: active".to_string(),
        ]
        .join("\n"),
    );

    Ok(())
}

pub(crate) fn build_memory_remember_write_report(
    content: &str,
) -> Result<(String, serde_json::Value), String> {
    if content.trim().is_empty() {
        return Err("memory.remember content 不能为空".to_string());
    }
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_main_memory_scope(),
            content: content.to_string(),
            localized_summary: content.to_string(),
            state_key: None,
            current_state: None,
        })
        .map_err(memory_error)?;

    Ok((
        "memory.remember write applied".to_string(),
        serde_json::json!({
            "operation": "memory.remember",
            "write_applied": "true",
            "audit_closed": "true",
            "write_outcome": "applied",
            "idempotency": "not_guaranteed",
            "memory_id": memory_id,
            "memory_type": "core",
            "source": "user_explicit",
            "permanence": "standard",
            "content_present": "true",
            "content_length": content.chars().count().to_string(),
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_accept_write_report(
    proposal_id: &str,
) -> Result<(String, serde_json::Value), String> {
    if proposal_id.trim().is_empty() {
        return Err("memory.accept proposal_id 不能为空".to_string());
    }
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .accept_proposal(proposal_id, "user", Some("cli accept"))
        .map_err(memory_error)?;

    Ok((
        "memory.accept write applied".to_string(),
        serde_json::json!({
            "operation": "memory.accept",
            "write_applied": "true",
            "audit_closed": "true",
            "write_outcome": "applied",
            "idempotency": "not_guaranteed",
            "proposal_id": proposal_id,
            "memory_id": memory_id,
            "status": "accepted",
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_reject_write_report(
    proposal_id: &str,
) -> Result<(String, serde_json::Value), String> {
    if proposal_id.trim().is_empty() {
        return Err("memory.reject proposal_id 不能为空".to_string());
    }
    let mut kernel = real_memory_kernel()?;
    kernel
        .reject_proposal(proposal_id, "user", Some("cli reject"))
        .map_err(memory_error)?;

    Ok((
        "memory.reject write applied".to_string(),
        serde_json::json!({
            "operation": "memory.reject",
            "write_applied": "true",
            "audit_closed": "true",
            "write_outcome": "applied",
            "idempotency": "not_guaranteed",
            "proposal_id": proposal_id,
            "status": "rejected",
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn memory_write_failure_fields(
    operation: &str,
    proposal_id: Option<String>,
    content_length: Option<usize>,
) -> serde_json::Value {
    let mut fields = serde_json::json!({
        "operation": operation,
        "write_applied": "false",
        "audit_closed": "true",
        "write_outcome": "failed",
        "idempotency": "not_guaranteed",
        "kernel_invocation_path": "binary"
    });
    let object = fields
        .as_object_mut()
        .expect("memory write failure fields should be an object");
    if let Some(proposal_id) = proposal_id.filter(|value| !value.trim().is_empty()) {
        object.insert(
            "proposal_id".to_string(),
            serde_json::Value::String(proposal_id),
        );
    }
    if let Some(content_length) = content_length {
        object.insert(
            "content_present".to_string(),
            serde_json::Value::String((content_length > 0).to_string()),
        );
        object.insert(
            "content_length".to_string(),
            serde_json::Value::String(content_length.to_string()),
        );
    }
    fields
}

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
