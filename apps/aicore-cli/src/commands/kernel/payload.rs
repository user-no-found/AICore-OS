use aicore_kernel::KernelEventPayload;
use aicore_terminal::{Block, Document};

use crate::terminal::{cli_row, emit_document};

pub(crate) fn emit_kernel_invocation_payload_json(payload: &serde_json::Value) {
    let payload = serde_json::to_string(payload).expect("kernel invocation result should encode");
    emit_document(Document::new(vec![Block::structured_json(
        "kernel.invocation.result",
        &payload,
    )]));
}

pub(crate) fn kernel_invocation_payload_rows(
    payload: &serde_json::Value,
    requested_operation: &str,
) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    rows.push(cli_row(
        "invocation",
        payload_status(payload).unwrap_or("failed").to_string(),
    ));
    push_payload_row(&mut rows, payload, &["invocation_id"], "invocation id");
    rows.push(cli_row("route", route_status_from_payload(payload)));
    rows.push(cli_row(
        "operation",
        string_at(payload, &["operation"]).unwrap_or(requested_operation),
    ));
    push_payload_row(&mut rows, payload, &["route", "component_id"], "component");
    push_payload_row(&mut rows, payload, &["route", "app_id"], "app");
    push_payload_row(
        &mut rows,
        payload,
        &["route", "capability_id"],
        "capability",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["route", "contract_version"],
        "contract",
    );
    push_payload_row(&mut rows, payload, &["handler", "kind"], "handler kind");
    push_payload_row(
        &mut rows,
        payload,
        &["handler", "invocation_mode"],
        "invocation mode",
    );
    push_payload_row(&mut rows, payload, &["handler", "transport"], "transport");
    push_payload_row(
        &mut rows,
        payload,
        &["handler", "executed"],
        "handler executed",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["handler", "event_generated"],
        "event generated",
    );
    push_payload_row(&mut rows, payload, &["result", "kind"], "result kind");
    push_payload_row(&mut rows, payload, &["result", "summary"], "result summary");
    push_payload_row(
        &mut rows,
        payload,
        &["ledger", "appended"],
        "ledger appended",
    );
    push_payload_row(&mut rows, payload, &["ledger", "path"], "ledger path");
    push_payload_row(&mut rows, payload, &["ledger", "records"], "ledger records");
    push_payload_row(
        &mut rows,
        payload,
        &["handler", "spawned_process"],
        "spawned process",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["handler", "called_real_component"],
        "called real component",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["handler", "first_party_in_process_adapter"],
        "first-party in-process adapter",
    );
    rows.push(cli_row("in-process fallback", "false"));

    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "foundation_installed"],
        "foundation installed",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "kernel_installed"],
        "kernel installed",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "manifest_count"],
        "manifest count",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "capability_count"],
        "capability count",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "foundation_runtime_binary"],
        "foundation runtime binary",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "kernel_runtime_binary"],
        "kernel runtime binary",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "kernel_invocation_path"],
        "kernel invocation path",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "protocol"],
        "protocol",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "protocol_version"],
        "protocol version",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "runtime_binary_contract_version"],
        "runtime contract",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["result", "fields", "binary_health"],
        "binary health",
    );
    push_additional_result_field_rows(&mut rows, payload);
    push_payload_row(
        &mut rows,
        payload,
        &["runtime_binary", "foundation_path"],
        "foundation runtime binary path",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["runtime_binary", "kernel_path"],
        "kernel runtime binary path",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["runtime_binary", "protocol"],
        "protocol",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["runtime_binary", "protocol_version"],
        "protocol version",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["runtime_binary", "contract_version"],
        "runtime contract",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["runtime_binary", "foundation_health"],
        "foundation runtime health",
    );
    push_payload_row(
        &mut rows,
        payload,
        &["runtime_binary", "kernel_health"],
        "kernel runtime health",
    );

    if payload_status(payload) != Some("completed") {
        push_payload_row(&mut rows, payload, &["failure", "stage"], "failure stage");
        push_payload_row(&mut rows, payload, &["failure", "reason"], "reason");
    } else {
        rows.push(cli_row(
            "说明",
            "通过 installed Kernel runtime binary 执行，不使用应用私有 in-process fallback",
        ));
    }

    rows
}

pub(crate) fn payload_status(payload: &serde_json::Value) -> Option<&str> {
    string_at(payload, &["status"])
}

pub(crate) fn event_payload_summary(payload: &KernelEventPayload) -> String {
    match payload {
        KernelEventPayload::Summary(summary) => summary.clone(),
        KernelEventPayload::Empty => "empty".to_string(),
    }
}

fn push_additional_result_field_rows(
    rows: &mut Vec<(String, String)>,
    payload: &serde_json::Value,
) {
    let Some(fields) = payload
        .get("result")
        .and_then(|result| result.get("fields"))
        .and_then(|fields| fields.as_object())
    else {
        return;
    };
    let known = [
        "foundation_installed",
        "kernel_installed",
        "manifest_count",
        "capability_count",
        "foundation_runtime_binary",
        "kernel_runtime_binary",
        "kernel_invocation_path",
        "protocol",
        "protocol_version",
        "runtime_binary_contract_version",
        "binary_health",
    ];
    for (key, value) in fields {
        if known.contains(&key.as_str()) {
            continue;
        }
        rows.push(cli_row(
            format!("result.{key}"),
            public_json_value(value).unwrap_or_else(|| "-".to_string()),
        ));
    }
}

fn push_payload_row(
    rows: &mut Vec<(String, String)>,
    payload: &serde_json::Value,
    path: &[&str],
    label: &str,
) {
    if let Some(value) = public_value_at(payload, path) {
        rows.push(cli_row(label, value));
    }
}

fn route_status_from_payload(payload: &serde_json::Value) -> &'static str {
    if payload.get("route").is_some_and(|route| route.is_object()) {
        "routed"
    } else {
        "failed"
    }
}

fn string_at<'a>(payload: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    let mut current = payload;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn public_value_at(payload: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = payload;
    for key in path {
        current = current.get(*key)?;
    }
    public_json_value(current)
}

fn public_json_value(current: &serde_json::Value) -> Option<String> {
    match current {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        value => Some(value.to_string()),
    }
}
