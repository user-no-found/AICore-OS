use std::path::Path;

use aicore_foundation::AicoreLayout;

use crate::KernelInvocationEnvelope;

use super::health::binary_health;
use super::protocol::{
    RUNTIME_BINARY_CONTRACT_VERSION, RUNTIME_BINARY_PROTOCOL, RUNTIME_BINARY_PROTOCOL_VERSION,
};
use super::request::KernelRuntimeBinaryRequest;
use super::response::KernelRuntimeBinaryResponse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelRuntimeBinaryErrorKind {
    FoundationBinaryMissing,
    FoundationBinaryNotExecutable,
    KernelBinaryMissing,
    KernelBinaryNotExecutable,
    ProcessSpawnFailed,
    StdinWriteFailed,
    StdoutReadFailed,
    NonZeroExit,
    InvalidJsonlOutput,
    ProtocolVersionMismatch,
    ContractVersionMismatch,
    KernelInvocationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryError {
    pub kind: KernelRuntimeBinaryErrorKind,
    pub stage: String,
    pub message: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryInvocation {
    pub request: KernelRuntimeBinaryRequest,
    pub response: Option<KernelRuntimeBinaryResponse>,
    pub payload: serde_json::Value,
    pub exit_success: bool,
    pub error: Option<KernelRuntimeBinaryError>,
}

pub(super) fn runtime_binary_failure_payload(
    envelope: &KernelInvocationEnvelope,
    stage: &str,
    reason: &str,
    layout: &AicoreLayout,
    foundation_binary: &Path,
    kernel_binary: &Path,
    in_process_fallback: bool,
    spawned_process: bool,
    process_exit_code: Option<i32>,
) -> serde_json::Value {
    let foundation_health = binary_health(foundation_binary);
    let kernel_health = binary_health(kernel_binary);
    serde_json::json!({
        "invocation_id": envelope.invocation_id,
        "trace_id": envelope.trace_context.trace_id,
        "operation": envelope.operation,
        "status": "failed",
        "route": serde_json::Value::Null,
        "handler": {
            "kind": "kernel_runtime_binary",
            "invocation_mode": "local_process",
            "transport": RUNTIME_BINARY_PROTOCOL,
            "process_exit_code": process_exit_code,
            "executed": false,
            "event_generated": false,
            "spawned_process": spawned_process,
            "called_real_component": false,
            "first_party_in_process_adapter": false,
        },
        "ledger": {
            "appended": false,
            "path": layout.kernel_state_root.join("invocation-ledger.jsonl").display().to_string(),
            "records": 0,
        },
        "result": {
            "kind": serde_json::Value::Null,
            "summary": serde_json::Value::Null,
            "fields": {},
        },
        "failure": {
            "stage": stage,
            "reason": sanitize_runtime_binary_diagnostic(reason),
        },
        "runtime_binary": {
            "foundation_path": foundation_binary.display().to_string(),
            "foundation_installed": foundation_binary.exists(),
            "foundation_health": foundation_health,
            "kernel_path": kernel_binary.display().to_string(),
            "kernel_installed": kernel_binary.exists(),
            "kernel_health": kernel_health,
            "protocol": RUNTIME_BINARY_PROTOCOL,
            "protocol_version": RUNTIME_BINARY_PROTOCOL_VERSION,
            "contract_version": RUNTIME_BINARY_CONTRACT_VERSION,
            "in_process_fallback": in_process_fallback,
        }
    })
}

pub(super) fn sanitize_runtime_binary_diagnostic(value: &str) -> String {
    let without_control = value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .collect::<String>();
    let mut summary = crate::redact_failure_reason(&without_control).replace('\n', " ");
    if summary.chars().count() > 240 {
        summary = summary.chars().take(240).collect::<String>();
        summary.push_str("...");
    }
    summary
}
