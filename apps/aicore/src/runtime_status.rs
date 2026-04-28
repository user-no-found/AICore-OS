use aicore_foundation::AicoreLayout;
use aicore_kernel::{
    InstalledManifestRegistry, KernelHandlerRegistry, KernelInvocationEnvelope,
    KernelInvocationLedger, KernelInvocationRuntime, KernelInvocationRuntimeOutput,
    KernelInvocationStatus, KernelPayload, runtime_status_handler_for_layout,
};

pub fn invoke_runtime_status(layout: &AicoreLayout) -> KernelInvocationRuntimeOutput {
    let registry = InstalledManifestRegistry::load_from_dir(&layout.manifests_root)
        .unwrap_or_else(|_| InstalledManifestRegistry::from_manifests(Vec::new()));
    let handlers = KernelHandlerRegistry::new().with_handler(
        "runtime.status",
        runtime_status_handler_for_layout(layout.clone()),
    );
    let runtime = KernelInvocationRuntime::new(registry, handlers);
    let ledger_path = layout.kernel_state_root.join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    runtime.invoke_with_ledger(
        KernelInvocationEnvelope::new(
            "global-main",
            "runtime.status",
            "runtime.status",
            KernelPayload::Empty,
        ),
        &ledger,
    )
}

pub fn runtime_status_rows(output: &KernelInvocationRuntimeOutput) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    match output.status {
        KernelInvocationStatus::Completed => {
            let result = output
                .result
                .as_ref()
                .expect("completed runtime status should include result envelope");
            rows.push(row("invocation", "completed"));
            rows.push(row("invocation id", result.invocation_id.as_str()));
            rows.push(row("operation", result.operation.as_str()));
            rows.push(row("handler executed", output.handler_executed.to_string()));
            rows.push(row("event generated", output.event_generated.to_string()));
            rows.push(row("ledger appended", output.ledger_appended.to_string()));
            rows.push(row(
                "ledger records",
                output.ledger_record_count.to_string(),
            ));
            push_field(&mut rows, result, "global_root", "global root");
            push_field(
                &mut rows,
                result,
                "foundation_installed",
                "foundation installed",
            );
            push_field(&mut rows, result, "kernel_installed", "kernel installed");
            push_field(&mut rows, result, "contract_version", "contract version");
            push_field(&mut rows, result, "manifest_count", "manifest count");
            push_field(&mut rows, result, "capability_count", "capability count");
            push_field(&mut rows, result, "event_ledger_path", "event ledger");
            push_field(&mut rows, result, "bin_path", "bin path");
            push_field(&mut rows, result, "bin_path_status", "bin path status");
        }
        KernelInvocationStatus::Failed => {
            rows.push(row("invocation", "failed"));
            rows.push(row("operation", "runtime.status"));
            rows.push(row(
                "failure stage",
                output.failure_stage.as_deref().unwrap_or("-"),
            ));
            rows.push(row(
                "reason",
                output
                    .failure_reason
                    .as_deref()
                    .unwrap_or("unknown failure"),
            ));
            rows.push(row("handler executed", output.handler_executed.to_string()));
            rows.push(row("event generated", output.event_generated.to_string()));
            rows.push(row("ledger appended", output.ledger_appended.to_string()));
            rows.push(row(
                "ledger records",
                output.ledger_record_count.to_string(),
            ));
        }
    }
    rows
}

pub fn kernel_invocation_result_json(output: &KernelInvocationRuntimeOutput) -> serde_json::Value {
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
        .unwrap_or(serde_json::Value::Null);
    let fields = result
        .map(|result| serde_json::json!(result.public_fields))
        .unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "invocation_id": result.map(|result| result.invocation_id.as_str()),
        "trace_id": result.map(|result| result.trace_id.as_str()),
        "operation": result
            .map(|result| result.operation.as_str())
            .or_else(|| output.route.as_ref().map(|route| route.operation.as_str())),
        "status": match output.status {
            KernelInvocationStatus::Completed => "completed",
            KernelInvocationStatus::Failed => "failed",
        },
        "route": route,
        "handler": {
            "kind": output.handler_kind.as_deref(),
            "executed": output.handler_executed,
            "event_generated": output.event_generated,
            "spawned_process": output.spawned_process,
            "called_real_component": output.called_real_component,
            "first_party_in_process_adapter": result
                .and_then(|result| result.result_kind.as_deref())
                == Some("runtime.status"),
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

fn push_field(
    rows: &mut Vec<(String, String)>,
    result: &aicore_kernel::KernelInvocationResultEnvelope,
    key: &str,
    label: &str,
) {
    if let Some(value) = result.public_fields.get(key) {
        rows.push(row(label, value));
    }
}

fn row(key: impl Into<String>, value: impl Into<String>) -> (String, String) {
    (key.into(), value.into())
}
