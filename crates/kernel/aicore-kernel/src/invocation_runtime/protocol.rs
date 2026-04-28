use crate::{
    KernelInvocationEnvelope, KernelRouteRuntimeOutput, format_contract, redact_failure_reason,
};

pub(super) fn local_ipc_request_json(
    envelope: &KernelInvocationEnvelope,
    route: &KernelRouteRuntimeOutput,
) -> String {
    serde_json::json!({
        "schema_version": "aicore.local_ipc.invocation.v1",
        "invocation_id": envelope.invocation_id,
        "trace_id": envelope.trace_context.trace_id,
        "instance_id": envelope.instance_id,
        "operation": envelope.operation,
        "route": {
            "component_id": route.component_id,
            "app_id": route.app_id,
            "capability_id": route.capability_id,
            "contract_version": format_contract(&route.contract_version),
            "invocation_mode": route.invocation_mode.as_str(),
            "transport": route.transport.as_str(),
        }
    })
    .to_string()
}

pub(super) fn json_value_to_public_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

pub(super) fn sanitize_process_diagnostic(value: &str) -> String {
    let without_control = value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .collect::<String>();
    let redacted = redact_failure_reason(&without_control);
    let mut summary = redacted.replace('\n', " ");
    if summary.chars().count() > 240 {
        summary = summary.chars().take(240).collect::<String>();
        summary.push_str("...");
    }
    summary
}
