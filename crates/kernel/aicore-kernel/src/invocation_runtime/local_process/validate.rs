use crate::KernelInvocationEnvelope;

use super::ComponentProcessFailure;

pub(super) fn parse_component_process_result(
    stdout: &str,
    envelope: &KernelInvocationEnvelope,
    exit_code: Option<i32>,
) -> Result<serde_json::Value, ComponentProcessFailure> {
    let lines = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(process_failure(
            "process_stdout_failed",
            "component process returned empty stdio_jsonl result",
            exit_code,
        ));
    }
    if lines.len() != 1 {
        return Err(process_failure(
            "process_protocol_mismatch",
            "component process stdout must contain exactly one JSONL result line",
            exit_code,
        ));
    }
    let value: serde_json::Value = serde_json::from_str(lines[0]).map_err(|error| {
        process_failure(
            "process_invalid_json",
            &format!("component process returned invalid JSON result: {error}"),
            exit_code,
        )
    })?;
    validate_component_process_result(&value, envelope, exit_code)?;
    Ok(value)
}

fn validate_component_process_result(
    value: &serde_json::Value,
    envelope: &KernelInvocationEnvelope,
    exit_code: Option<i32>,
) -> Result<(), ComponentProcessFailure> {
    if value.get("schema_version").and_then(|value| value.as_str())
        != Some(super::super::protocol::LOCAL_IPC_RESULT_SCHEMA_VERSION)
    {
        return Err(process_failure(
            "process_result_schema_mismatch",
            "component process result schema mismatch",
            exit_code,
        ));
    }
    if value.get("protocol").and_then(|value| value.as_str())
        != Some(super::super::protocol::LOCAL_IPC_PROTOCOL)
        || value
            .get("protocol_version")
            .and_then(|value| value.as_str())
            != Some(super::super::protocol::LOCAL_IPC_PROTOCOL_VERSION)
    {
        return Err(process_failure(
            "process_protocol_mismatch",
            "component process result protocol mismatch",
            exit_code,
        ));
    }
    if value.get("invocation_id").and_then(|value| value.as_str())
        != Some(envelope.invocation_id.as_str())
    {
        return Err(process_failure(
            "process_result_mismatch",
            "component process result invocation_id mismatch",
            exit_code,
        ));
    }
    if value.get("status").and_then(|value| value.as_str()) != Some("completed") {
        return Err(process_failure(
            "process_result_schema_mismatch",
            "component process result status must be completed",
            exit_code,
        ));
    }
    if value
        .get("result_kind")
        .and_then(|value| value.as_str())
        .is_none()
        || value
            .get("summary")
            .and_then(|value| value.as_str())
            .is_none()
    {
        return Err(process_failure(
            "process_result_schema_mismatch",
            "component process result requires result_kind and summary",
            exit_code,
        ));
    }
    Ok(())
}

fn process_failure(stage: &str, reason: &str, exit_code: Option<i32>) -> ComponentProcessFailure {
    ComponentProcessFailure {
        stage: stage.to_string(),
        reason: super::super::protocol::sanitize_process_diagnostic(reason),
        spawned_process: true,
        exit_code,
    }
}
