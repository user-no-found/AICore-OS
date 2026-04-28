use std::fs;
use std::path::PathBuf;

use crate::{
    KernelHandlerRegistry, KernelInvocationEnvelope, KernelInvocationLedger,
    KernelInvocationStatus, KernelPayload,
};

use super::helpers::{
    envelope, failing_handler, process_fixture_script, registry_with_manifest,
    runtime_with_handler, secret_failing_handler, smoke_handler, structured_secret_result_handler,
    write_process_manifest,
};

#[test]
fn component_process_smoke_writes_invocation_ledger() {
    let root = super::helpers::temp_dir("process-smoke-ledger");
    let script = process_fixture_script(
        &root,
        "process-smoke-ledger.sh",
        r#"read line
printf '{"result_kind":"component.process.smoke","summary":"process smoke ok","fields":{"operation":"component.process.smoke"}}\n'
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let ledger_path =
        super::helpers::temp_dir("process-smoke-ledger-path").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke_with_ledger(envelope("component.process.smoke"), &ledger);
    let joined = read_ledger_records(&ledger_path).join("\n");

    assert_eq!(output.status, KernelInvocationStatus::Completed);
    assert_eq!(output.ledger_record_count, 5);
    assert!(joined.contains("\"handler_kind\":\"local_process\""));
    assert!(joined.contains("\"spawned_process\":true"));
    assert!(joined.contains("\"transport\":\"stdio_jsonl\""));
    assert!(!joined.contains("process smoke ok"));
}

#[test]
fn invocation_ledger_appends_accepted_and_completed_records() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path = super::helpers::temp_dir("ledger-success").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    let records = read_ledger_records(&ledger_path);

    assert_eq!(output.status, KernelInvocationStatus::Completed);
    assert!(output.ledger_appended);
    assert_eq!(output.ledger_record_count, 5);
    assert_eq!(
        ledger_stages(&records),
        vec![
            "accepted",
            "route_decision_made",
            "handler_executed",
            "event_generated",
            "invocation_completed",
        ]
    );
}

#[test]
fn invocation_ledger_appends_route_failure_record() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-route-failure").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke_with_ledger(envelope("unknown.operation"), &ledger);
    let records = read_ledger_records(&ledger_path);

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert!(!output.handler_executed);
    assert_eq!(
        ledger_stages(&records),
        vec!["accepted", "route_failed", "invocation_failed"]
    );
    assert!(
        records
            .iter()
            .any(|record| record.contains("missing capability"))
    );
}

#[test]
fn invocation_ledger_appends_missing_handler_failure_record() {
    let registry = registry_with_manifest(&[("provider.smoke", "provider.smoke")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-missing-handler").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(&registry, KernelHandlerRegistry::new());

    let output = runtime.invoke_with_ledger(envelope("provider.smoke"), &ledger);
    let records = read_ledger_records(&ledger_path);

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(
        ledger_stages(&records),
        vec![
            "accepted",
            "route_decision_made",
            "handler_lookup_failed",
            "invocation_failed",
        ]
    );
}

#[test]
fn invocation_ledger_appends_handler_failure_record() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-handler-failure").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", failing_handler),
    );

    let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    let records = read_ledger_records(&ledger_path);

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert!(output.handler_executed);
    assert_eq!(
        ledger_stages(&records),
        vec![
            "accepted",
            "route_decision_made",
            "handler_failed",
            "invocation_failed",
        ]
    );
}

#[test]
fn invocation_ledger_records_trace_and_invocation_ids() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path = super::helpers::temp_dir("ledger-trace").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    let joined = read_ledger_records(&ledger_path).join("\n");

    assert!(joined.contains("\"trace_id\":\"trace.default\""));
    assert!(joined.contains("\"invocation_id\":\"invoke."));
    assert!(!joined.contains("\"invocation_id\":\"invoke.memory.search\""));
    assert!(joined.contains("\"instance_id\":\"global-main\""));
}

#[test]
fn invocation_ledger_uses_same_invocation_id_for_one_invocation() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-same-invocation-id").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

    assert_eq!(ids.len(), 5);
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_ne!(ids[0], "invoke.memory.search");
}

#[test]
fn invocation_ledger_uses_distinct_invocation_id_for_repeated_same_operation() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-distinct-invocation-id").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

    assert_eq!(ids.len(), 10);
    assert_eq!(ids[0], ids[4]);
    assert_eq!(ids[5], ids[9]);
    assert_ne!(ids[0], ids[5]);
}

#[test]
fn invocation_route_failure_records_share_same_invocation_id() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-route-failure-id").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    runtime.invoke_with_ledger(envelope("unknown.operation"), &ledger);
    let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

    assert_eq!(ids.len(), 3);
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_ne!(ids[0], "invoke.unknown.operation");
}

#[test]
fn invocation_missing_handler_records_share_same_invocation_id() {
    let registry = registry_with_manifest(&[("provider.smoke", "provider.smoke")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-missing-handler-id").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(&registry, KernelHandlerRegistry::new());

    runtime.invoke_with_ledger(envelope("provider.smoke"), &ledger);
    let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

    assert_eq!(ids.len(), 4);
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_ne!(ids[0], "invoke.provider.smoke");
}

#[test]
fn invocation_handler_failure_records_share_same_invocation_id() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-handler-failure-id").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", failing_handler),
    );

    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

    assert_eq!(ids.len(), 4);
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_ne!(ids[0], "invoke.memory.search");
}

#[test]
fn invocation_ledger_is_json_lines() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path = super::helpers::temp_dir("ledger-jsonl").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

    for record in read_ledger_records(&ledger_path) {
        assert!(record.starts_with('{'));
        assert!(record.ends_with('}'));
        assert!(record.contains("\"schema_version\":\"aicore.kernel.invocation_ledger.v1\""));
    }
}

#[test]
fn invocation_ledger_is_append_only() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-append-only").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

    assert_eq!(read_ledger_records(&ledger_path).len(), 10);
}

#[test]
fn invocation_ledger_does_not_record_raw_payload() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path = super::helpers::temp_dir("ledger-no-payload").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    runtime.invoke_with_ledger(
        KernelInvocationEnvelope::new(
            "global-main",
            "memory.search",
            "memory.search",
            KernelPayload::Text("raw memory content should not be written".to_string()),
        ),
        &ledger,
    );
    let joined = read_ledger_records(&ledger_path).join("\n");

    assert!(!joined.contains("raw memory content should not be written"));
    assert!(!joined.contains("Text("));
}

#[test]
fn kernel_invocation_ledger_does_not_record_raw_result_payload() {
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-no-result-payload").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new()
            .with_handler("runtime.status", structured_secret_result_handler),
    );

    runtime.invoke_with_ledger(envelope("runtime.status"), &ledger);
    let joined = read_ledger_records(&ledger_path).join("\n");

    assert!(!joined.contains("structured-secret-field-value"));
    assert!(!joined.contains("raw result payload"));
    assert!(!joined.contains("secret_ref"));
}

#[test]
fn invocation_ledger_redacts_secret_like_values() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path = super::helpers::temp_dir("ledger-redaction").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", secret_failing_handler),
    );

    runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
    let joined = read_ledger_records(&ledger_path).join("\n");

    assert!(!joined.contains("sk-live-secret-value"));
    assert!(!joined.contains("secret://auth.openai.main"));
    assert!(!joined.contains("token=abc123"));
    assert!(joined.contains("[redacted"));
}

#[test]
fn invocation_ledger_append_failure_before_route_does_not_route_or_execute_handler() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-fail-before-route").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::failing_for_test(&ledger_path, "accepted");
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(output.failure_stage.as_deref(), Some("ledger_append"));
    assert!(!output.route_decision_made);
    assert!(!output.handler_executed);
    assert!(!output.ledger_appended);
}

#[test]
fn invocation_runtime_returns_failure_when_ledger_append_fails() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-fail-after-route").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::failing_for_test(&ledger_path, "handler_executed");
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(output.failure_stage.as_deref(), Some("ledger_append"));
    assert!(output.route_decision_made);
    assert!(output.handler_executed);
    assert!(!output.event_generated);
    assert!(!output.ledger_appended);
}

#[test]
fn invocation_runtime_completed_ledger_append_failure_reports_action_happened() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let ledger_path =
        super::helpers::temp_dir("ledger-fail-completed").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::failing_for_test(&ledger_path, "invocation_completed");
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(output.failure_stage.as_deref(), Some("ledger_append"));
    assert!(output.handler_executed);
    assert!(output.event_generated);
    assert!(!output.ledger_appended);
    assert!(
        output
            .failure_reason
            .as_deref()
            .expect("failure reason")
            .contains("audit close failed after action happened")
    );
}

pub(super) fn read_ledger_records(path: &PathBuf) -> Vec<String> {
    fs::read_to_string(path)
        .expect("ledger should be readable")
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

fn ledger_stages(records: &[String]) -> Vec<&'static str> {
    let all_stages = [
        "accepted",
        "route_decision_made",
        "route_failed",
        "handler_lookup_failed",
        "handler_failed",
        "handler_executed",
        "event_generated",
        "invocation_completed",
        "invocation_failed",
    ];
    records
        .iter()
        .map(|record| {
            all_stages
                .iter()
                .copied()
                .find(|stage| record.contains(&format!("\"stage\":\"{stage}\"")))
                .expect("known stage should exist")
        })
        .collect()
}

fn ledger_invocation_ids(records: &[String]) -> Vec<String> {
    records
        .iter()
        .map(|record| extract_json_string(record, "invocation_id"))
        .collect()
}

fn extract_json_string(record: &str, key: &str) -> String {
    let marker = format!("\"{key}\":\"");
    let start = record.find(&marker).expect("key should exist") + marker.len();
    let tail = &record[start..];
    let end = tail.find('"').expect("value should end");
    tail[..end].to_string()
}
