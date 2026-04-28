use aicore_foundation::AicoreLayout;
use aicore_kernel::KernelRuntimeBinaryClient;

pub fn invoke_runtime_status(layout: &AicoreLayout) -> serde_json::Value {
    KernelRuntimeBinaryClient::new(layout.clone())
        .invoke_readonly("runtime.status")
        .payload
}

pub fn runtime_status_rows(payload: &serde_json::Value) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    let status = string_at(payload, &["status"]).unwrap_or("failed");
    match status {
        "completed" => {
            rows.push(row("invocation", "completed"));
            push_string(&mut rows, payload, &["invocation_id"], "invocation id");
            push_string(&mut rows, payload, &["operation"], "operation");
            push_string(
                &mut rows,
                payload,
                &["handler", "executed"],
                "handler executed",
            );
            push_string(
                &mut rows,
                payload,
                &["handler", "event_generated"],
                "event generated",
            );
            push_string(
                &mut rows,
                payload,
                &["ledger", "appended"],
                "ledger appended",
            );
            push_string(&mut rows, payload, &["ledger", "records"], "ledger records");
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "global_root"],
                "global root",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "foundation_installed"],
                "foundation installed",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "kernel_installed"],
                "kernel installed",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "contract_version"],
                "contract version",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "manifest_count"],
                "manifest count",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "capability_count"],
                "capability count",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "event_ledger_path"],
                "event ledger",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "bin_path"],
                "bin path",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "bin_path_status"],
                "bin path status",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "foundation_runtime_binary"],
                "foundation runtime binary",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "kernel_runtime_binary"],
                "kernel runtime binary",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "kernel_invocation_path"],
                "kernel invocation path",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "protocol"],
                "protocol",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "protocol_version"],
                "protocol version",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "runtime_binary_contract_version"],
                "runtime contract",
            );
            push_string(
                &mut rows,
                payload,
                &["result", "fields", "binary_health"],
                "binary health",
            );
            rows.push(row("in-process fallback", "false"));
        }
        _ => {
            rows.push(row("invocation", "failed"));
            push_string(&mut rows, payload, &["operation"], "operation");
            push_string(&mut rows, payload, &["failure", "stage"], "failure stage");
            push_string(&mut rows, payload, &["failure", "reason"], "reason");
            push_string(
                &mut rows,
                payload,
                &["handler", "executed"],
                "handler executed",
            );
            push_string(
                &mut rows,
                payload,
                &["handler", "event_generated"],
                "event generated",
            );
            push_string(
                &mut rows,
                payload,
                &["ledger", "appended"],
                "ledger appended",
            );
            push_string(&mut rows, payload, &["ledger", "records"], "ledger records");
            push_string(
                &mut rows,
                payload,
                &["runtime_binary", "foundation_path"],
                "foundation runtime binary path",
            );
            push_string(
                &mut rows,
                payload,
                &["runtime_binary", "kernel_path"],
                "kernel runtime binary path",
            );
            push_string(
                &mut rows,
                payload,
                &["runtime_binary", "protocol"],
                "protocol",
            );
            push_string(
                &mut rows,
                payload,
                &["runtime_binary", "protocol_version"],
                "protocol version",
            );
            push_string(
                &mut rows,
                payload,
                &["runtime_binary", "contract_version"],
                "runtime contract",
            );
            push_string(
                &mut rows,
                payload,
                &["runtime_binary", "foundation_health"],
                "foundation runtime health",
            );
            push_string(
                &mut rows,
                payload,
                &["runtime_binary", "kernel_health"],
                "kernel runtime health",
            );
            rows.push(row("in-process fallback", "false"));
        }
    }
    rows
}

pub fn kernel_invocation_result_json(payload: &serde_json::Value) -> serde_json::Value {
    payload.clone()
}

pub fn status_code(payload: &serde_json::Value) -> i32 {
    if string_at(payload, &["status"]) == Some("completed") {
        0
    } else {
        1
    }
}

fn push_string(
    rows: &mut Vec<(String, String)>,
    payload: &serde_json::Value,
    path: &[&str],
    label: &str,
) {
    if let Some(value) = string_at(payload, path) {
        rows.push(row(label, value));
    }
}

fn string_at<'a>(payload: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    let mut current = payload;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().or_else(|| match current {
        serde_json::Value::Bool(true) => Some("true"),
        serde_json::Value::Bool(false) => Some("false"),
        _ => None,
    })
}

fn row(key: impl Into<String>, value: impl Into<String>) -> (String, String) {
    (key.into(), value.into())
}
