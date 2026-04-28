use std::fs;

use crate::{KernelHandlerRegistry, KernelInvocationLedger, KernelInvocationStatus, TimeoutPolicy};

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
invocation_id=$(printf '%s' "$line" | sed -n 's/.*"invocation_id":"\([^"]*\)".*/\1/p')
printf '{"schema_version":"aicore.local_ipc.result.v1","protocol":"stdio_jsonl","protocol_version":"aicore.local_ipc.stdio_jsonl.v1","invocation_id":"%s","status":"completed","result_kind":"component.process.smoke","summary":"process smoke ok","fields":{"operation":"component.process.smoke","ipc":"stdio_jsonl"}}\n' "$invocation_id"
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
fn component_process_timeout_returns_structured_failure_and_kills_child() {
    let root = temp_dir("process-timeout");
    let pid_path = root.join("child.pid");
    let script = process_fixture_script(
        &root,
        "process-timeout.sh",
        &format!("read line\necho $$ > '{}'\nsleep 5\n", pid_path.display()),
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let ledger_path = temp_dir("process-timeout-ledger").join("invocation-ledger.jsonl");
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());
    let mut invocation = envelope("component.process.smoke");
    invocation.policy.timeout = TimeoutPolicy::Millis(50);

    let output = runtime.invoke_with_ledger(invocation, &ledger);
    let child_pid = fs::read_to_string(pid_path).expect("child pid should be written");
    let proc_path = format!("/proc/{}", child_pid.trim());

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(output.failure_stage.as_deref(), Some("process_timeout"));
    assert!(output.spawned_process);
    assert!(!std::path::Path::new(&proc_path).exists());
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
fn component_process_not_executable_is_structured_failure() {
    let root = temp_dir("process-not-executable");
    let script = root.join("not-executable.sh");
    fs::write(&script, "#!/bin/sh\nexit 0\n").expect("write fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&script, permissions).expect("set permissions");
    }
    write_process_manifest(
        &root,
        script.display().to_string().as_str(),
        "stdio_jsonl",
        &[],
    );
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("entrypoint_not_executable")
    );
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
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("process_non_zero_exit")
    );
    assert_eq!(output.process_exit_code, Some(42));
    assert!(output.spawned_process);
    assert!(!output.event_generated);
    assert!(!joined.contains("sk-live-secret-value"));
    assert!(!joined.contains("token=abc123"));
    assert!(joined.contains("[redacted"));
}

#[test]
fn component_process_empty_stdout_is_structured_failure() {
    let root = temp_dir("process-empty-stdout");
    let script = process_fixture_script(
        &root,
        "process-empty-stdout.sh",
        r#"read line
exit 0
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("process_stdout_failed")
    );
    assert!(output.spawned_process);
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
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("process_invalid_json")
    );
    assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
    assert!(output.spawned_process);
    assert!(!output.event_generated);
}

#[test]
fn component_process_protocol_mismatch_is_structured_failure() {
    let root = temp_dir("process-protocol-mismatch");
    let script = process_fixture_script(
        &root,
        "process-protocol-mismatch.sh",
        r#"read line
printf '{"schema_version":"aicore.local_ipc.result.v1","protocol":"stdio_jsonl","protocol_version":"wrong.version","invocation_id":"invoke.fixture","status":"completed","result_kind":"component.process.smoke","summary":"bad protocol","fields":{}}\n'
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("process_protocol_mismatch")
    );
    assert!(output.spawned_process);
}

#[test]
fn component_process_result_invocation_id_mismatch_is_structured_failure() {
    let root = temp_dir("process-invocation-mismatch");
    let script = process_fixture_script(
        &root,
        "process-invocation-mismatch.sh",
        r#"read line
printf '{"schema_version":"aicore.local_ipc.result.v1","protocol":"stdio_jsonl","protocol_version":"aicore.local_ipc.stdio_jsonl.v1","invocation_id":"invoke.wrong","status":"completed","result_kind":"component.process.smoke","summary":"wrong invocation","fields":{}}\n'
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("process_result_mismatch")
    );
    assert!(output.spawned_process);
}

#[test]
fn component_process_result_schema_mismatch_is_structured_failure() {
    let root = temp_dir("process-schema-mismatch");
    let script = process_fixture_script(
        &root,
        "process-schema-mismatch.sh",
        r#"read line
printf '{"result_kind":"component.process.smoke","summary":"legacy result","fields":{}}\n'
"#,
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert_eq!(
        output.failure_stage.as_deref(),
        Some("process_result_schema_mismatch")
    );
    assert!(output.spawned_process);
}

#[test]
fn component_process_stderr_is_redacted_and_truncated() {
    let root = temp_dir("process-stderr-redaction");
    let long_tail = "x".repeat(400);
    let script = process_fixture_script(
        &root,
        "process-stderr-redaction.sh",
        &format!(
            "read line\nprintf 'secret sk-live-secret-value token=abc123 {long_tail}\\n' >&2\nexit 7\n"
        ),
    );
    write_process_manifest(&root, &script, "stdio_jsonl", &[]);
    let runtime = runtime_with_handler(&root, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("component.process.smoke"));
    let reason = output.failure_reason.expect("failure reason");

    assert!(!reason.contains("sk-live-secret-value"));
    assert!(!reason.contains("token=abc123"));
    assert!(reason.contains("[redacted"));
    assert!(reason.len() <= 260);
}
