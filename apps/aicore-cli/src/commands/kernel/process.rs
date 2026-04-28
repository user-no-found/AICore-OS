use std::io::Read;

use aicore_foundation::AicoreLayout;
use aicore_kernel::{KernelInvocationEnvelope, KernelPayload, KernelRuntimeBinaryClient};
use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::terminal::{cli_row, emit_cli_panel};

use super::payload::{
    emit_kernel_invocation_payload_json, kernel_invocation_payload_rows, payload_status,
};

pub(crate) fn print_kernel_invoke_process_smoke(operation: &str) -> i32 {
    let layout = AicoreLayout::from_system_home();
    let envelope =
        KernelInvocationEnvelope::new("global-main", operation, operation, KernelPayload::Empty);
    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_envelope(envelope);
    let output = invocation.payload;

    if TerminalConfig::current().mode == TerminalMode::Json {
        emit_kernel_invocation_payload_json(&output);
        return if invocation.exit_success { 0 } else { 1 };
    }

    let title = if payload_status(&output) == Some("completed") {
        "内核组件进程调用 Smoke"
    } else {
        "内核组件进程调用失败"
    };
    let mut rows = kernel_invocation_payload_rows(&output, operation);
    if payload_status(&output) == Some("completed") {
        rows.push(cli_row(
            "边界",
            "只验证 local process boundary，不代表业务组件已迁移",
        ));
    }
    emit_cli_panel(title, rows);
    if invocation.exit_success { 0 } else { 1 }
}

pub(crate) fn run_component_smoke_stdio() -> i32 {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("component smoke stdin 读取失败: {error}");
        return 1;
    }
    let line = input
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("{}");
    let value: serde_json::Value =
        serde_json::from_str(line).unwrap_or_else(|_| serde_json::json!({}));
    let operation = value
        .get("operation")
        .and_then(|value| value.as_str())
        .unwrap_or("component.process.smoke");
    let invocation_id = value
        .get("invocation_id")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let result = serde_json::json!({
        "schema_version": "aicore.local_ipc.result.v1",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.local_ipc.stdio_jsonl.v1",
        "invocation_id": invocation_id,
        "status": "completed",
        "result_kind": "component.process.smoke",
        "summary": format!("process smoke handled {operation}"),
        "fields": {
            "operation": operation,
            "ipc": "stdio_jsonl",
            "invocation_id": invocation_id,
            "component_process": "ok"
        }
    });
    println!(
        "{}",
        serde_json::to_string(&result).expect("component smoke result should encode")
    );
    0
}
