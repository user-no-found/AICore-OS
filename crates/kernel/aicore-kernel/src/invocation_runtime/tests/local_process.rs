use crate::{KernelHandlerRegistry, KernelInvocationLedger, KernelInvocationStatus};

use super::helpers::{
    envelope, process_fixture_script, runtime_with_handler, temp_dir, write_process_manifest,
};

#[test]
fn component_process_smoke_invokes_stdio_jsonl_child() {
    let root = temp_dir("process-smoke-success");
    let script = process_fixture_script(
        &root,
        "process-smoke-success.sh",
        r#"read line
printf '{"result_kind":"component.process.smoke","summary":"process smoke ok","fields":{"operation":"component.process.smoke","ipc":"stdio_jsonl"}}\n'
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let ledger_path = temp_dir("process-smoke-success-ledger").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke_with_ledger(envelope("component.process.smoke"), &ledger);

    assert_eq!(output.status, KernelInvocationStatus::Completed);
    assert!(output.handler_executed);
    assert!(output.event_generated);
    assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
    assert_eq!(output.transport.as_deref(), Some("stdio_jsonl"));
    assert!(output.spawned_process);
    assert_eq!(output.process_exit_code, Some(0));
    let result = output.result.expect("process result envelope");
    assert_eq!(
        result.result_kind.as_deref(),
        Some("component.process.smoke")
    );
    assert_eq!(
        result.public_fields.get("ipc"),
        Some(&"stdio_jsonl".to_string())
    );
}

#[test]
fn component_process_unsupported_transport_returns_structured_failure() {
    let root = temp_dir("process-unsupported-transport");
    write_process_manifest(&root, "/bin/sh", "unix_socket", &[]);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("transport_unsupported")
    );
    assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
    assert!(!output.spawned_process);
    assert!(!output.event_generated);
}

#[test]
fn component_process_missing_entrypoint_returns_structured_failure() {
    let root = temp_dir("process-missing-entrypoint");
    write_process_manifest(
        &root,
        root.join("missing-component")
            .display()
            .to_string()
            .as_str(),
        "stdio_jsonl",
        &[],
    );
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(output.failure_stage.as_deref(), Some("missing_entrypoint"));
    assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
    assert!(!output.spawned_process);
}

#[test]
fn component_process_nonzero_exit_returns_structured_failure() {
    let root = temp_dir("process-nonzero");
    let script = process_fixture_script(
        &root,
        "process-nonzero.sh",
        r#"read line
printf 'failed with sk-live-secret-value token=abc123\n' >&2
exit 42
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let ledger_path = temp_dir("process-nonzero-ledger").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke_with_ledger(envelope("component.process.smoke"), &ledger);
    let joined = super::ledger::read_ledger_records(&ledger_path).join("\n");

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(output.failure_stage.as_deref(), Some("process_exit"));
    assert_eq!(output.process_exit_code, Some(42));
    assert!(output.spawned_process);
    assert!(!output.event_generated);
    assert!(!joined.contains("sk-live-secret-value"));
    assert!(!joined.contains("token=abc123"));
    assert!(joined.contains("[redacted"));
}

#[test]
fn component_process_invalid_json_returns_structured_failure() {
    let root = temp_dir("process-invalid-json");
    let script = process_fixture_script(
        &root,
        "process-invalid-json.sh",
        r#"read line
printf 'not json\n'
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(output.failure_stage.as_deref(), Some("ipc_read"));
    assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
    assert!(output.spawned_process);
    assert!(!output.event_generated);
}
