use crate::{KernelInvocationRuntimeOutput, KernelInvocationStatus};

use super::protocol::{
    RUNTIME_BINARY_CONTRACT_VERSION, RUNTIME_BINARY_PROTOCOL, RUNTIME_BINARY_PROTOCOL_VERSION,
};

pub fn kernel_invocation_result_public_json(
    output: &KernelInvocationRuntimeOutput,
) -> serde_json::Value {
    let result = output.result.as_ref();
    let route = result
        .and_then(|result| result.route.as_ref())
        .map(|route| {
            serde_json::json!({
                "component_id": route.component_id,
                "app_id": route.app_id,
                "capability_id": route.capability_id,
                "contract_version": route.contract_version,
            })
        })
        .or_else(|| {
            output.route.as_ref().map(|route| {
                serde_json::json!({
                    "component_id": route.component_id,
                    "app_id": route.app_id,
                    "capability_id": route.capability_id,
                    "contract_version": crate::format_contract(&route.contract_version),
                })
            })
        })
        .unwrap_or(serde_json::Value::Null);
    let mut fields = result
        .map(|result| serde_json::json!(result.public_fields))
        .unwrap_or_else(|| serde_json::json!({}));
    let via_runtime_binary = result
        .and_then(|result| result.public_fields.get("kernel_invocation_path"))
        .is_some_and(|value| value == "binary");
    let handler_kind = if via_runtime_binary {
        Some("kernel_runtime_binary")
    } else {
        output.handler_kind.as_deref()
    };
    let invocation_mode = if via_runtime_binary {
        Some("local_process")
    } else {
        output
            .route
            .as_ref()
            .map(|route| route.invocation_mode.as_str())
    };
    let transport = if via_runtime_binary {
        Some(RUNTIME_BINARY_PROTOCOL)
    } else {
        output.transport.as_deref()
    };
    let spawned_process = output.spawned_process || via_runtime_binary;
    if via_runtime_binary {
        add_runtime_binary_contract_metadata_to_fields(&mut fields);
    }

    serde_json::json!({
        "invocation_id": result
            .map(|result| result.invocation_id.as_str())
            .or_else(|| output.event.as_ref().map(|event| event.invocation_id.as_str())),
        "trace_id": result
            .map(|result| result.trace_id.as_str())
            .or_else(|| output.event.as_ref().map(|event| event.trace_context.trace_id.as_str())),
        "operation": result
            .map(|result| result.operation.as_str())
            .or_else(|| output.route.as_ref().map(|route| route.operation.as_str())),
        "status": match output.status {
            KernelInvocationStatus::Completed => "completed",
            KernelInvocationStatus::Failed => "failed",
        },
        "route": route,
        "handler": {
            "kind": handler_kind,
            "invocation_mode": invocation_mode,
            "transport": transport,
            "process_exit_code": output.process_exit_code,
            "executed": output.handler_executed,
            "event_generated": output.event_generated,
            "spawned_process": spawned_process,
            "called_real_component": output.called_real_component,
            "first_party_in_process_adapter": !via_runtime_binary
                && output.handler_kind.as_deref() == Some("in_process")
                && result.and_then(|result| result.result_kind.as_deref()) == Some("runtime.status"),
        },
        "ledger": {
            "appended": output.ledger_appended,
            "path": output.ledger_path.as_deref(),
            "records": output.ledger_record_count,
        },
        "result": {
            "kind": result.and_then(|result| result.result_kind.as_deref()),
            "summary": result.map(|result| result.summary.as_str()),
            "fields": fields,
        },
        "failure": {
            "stage": output.failure_stage.as_deref(),
            "reason": output.failure_reason.as_deref(),
        }
    })
}

pub(super) fn add_runtime_binary_contract_metadata(payload: &mut serde_json::Value) {
    if let Some(result_fields) = payload
        .get_mut("result")
        .and_then(|result| result.get_mut("fields"))
        .and_then(|fields| fields.as_object_mut())
    {
        insert_runtime_binary_contract_metadata(result_fields);
    }
}

fn add_runtime_binary_contract_metadata_to_fields(fields: &mut serde_json::Value) {
    if let Some(fields) = fields.as_object_mut() {
        insert_runtime_binary_contract_metadata(fields);
    }
}

fn insert_runtime_binary_contract_metadata(
    fields: &mut serde_json::Map<String, serde_json::Value>,
) {
    fields.insert(
        "kernel_invocation_path".to_string(),
        serde_json::Value::String("binary".to_string()),
    );
    fields.insert(
        "protocol".to_string(),
        serde_json::Value::String(RUNTIME_BINARY_PROTOCOL.to_string()),
    );
    fields.insert(
        "protocol_version".to_string(),
        serde_json::Value::String(RUNTIME_BINARY_PROTOCOL_VERSION.to_string()),
    );
    fields.insert(
        "runtime_binary_contract_version".to_string(),
        serde_json::Value::String(RUNTIME_BINARY_CONTRACT_VERSION.to_string()),
    );
    fields.insert(
        "binary_health".to_string(),
        serde_json::Value::String("ok".to_string()),
    );
}
