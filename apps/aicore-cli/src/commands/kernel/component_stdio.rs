use std::io::Read;

pub(crate) fn run_component_report_stdio(
    result_kind: &str,
    stdin_error: &str,
    build_report: impl FnOnce() -> Result<(String, serde_json::Value), String>,
) -> i32 {
    run_component_report_stdio_with_request(result_kind, stdin_error, |_| build_report())
}

pub(crate) fn run_component_report_stdio_with_request(
    result_kind: &str,
    stdin_error: &str,
    build_report: impl FnOnce(&serde_json::Value) -> Result<(String, serde_json::Value), String>,
) -> i32 {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("{stdin_error}: {error}");
        return 1;
    }
    let request = first_json_line(&input);
    let invocation_id = request
        .get("invocation_id")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let (summary, fields) = match build_report(&request) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{result_kind} component 执行失败: {error}");
            return 1;
        }
    };
    let result = serde_json::json!({
        "schema_version": "aicore.local_ipc.result.v1",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.local_ipc.stdio_jsonl.v1",
        "invocation_id": invocation_id,
        "status": "completed",
        "result_kind": result_kind,
        "summary": summary,
        "fields": fields
    });
    println!(
        "{}",
        serde_json::to_string(&result).expect("component readonly result should encode")
    );
    0
}

pub(crate) fn run_component_write_report_stdio_with_request(
    result_kind: &str,
    stdin_error: &str,
    failure_fields: impl FnOnce(&serde_json::Value, &str) -> serde_json::Value,
    build_report: impl FnOnce(&serde_json::Value) -> Result<(String, serde_json::Value), String>,
) -> i32 {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("{stdin_error}: {error}");
        return 1;
    }
    let request = first_json_line(&input);
    let invocation_id = request
        .get("invocation_id")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let (status, summary, fields) = match build_report(&request) {
        Ok((summary, fields)) => ("completed", summary, fields),
        Err(error) => {
            eprintln!("{result_kind} component 执行失败: {error}");
            ("failed", error.clone(), failure_fields(&request, &error))
        }
    };
    let result = serde_json::json!({
        "schema_version": "aicore.local_ipc.result.v1",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.local_ipc.stdio_jsonl.v1",
        "invocation_id": invocation_id,
        "status": status,
        "result_kind": result_kind,
        "summary": summary,
        "fields": fields
    });
    println!(
        "{}",
        serde_json::to_string(&result).expect("component write result should encode")
    );
    0
}

pub(crate) fn payload_string(request: &serde_json::Value, key: &str, default: &str) -> String {
    request
        .get("payload")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(default)
        .to_string()
}

pub(crate) fn first_json_line(input: &str) -> serde_json::Value {
    let line = input
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("{}");
    serde_json::from_str(line).unwrap_or_else(|_| serde_json::json!({}))
}
