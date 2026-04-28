use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use aicore_foundation::AicoreLayout;

use super::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn temp_layout(name: &str) -> AicoreLayout {
    let root = std::env::temp_dir().join(format!(
        "aicore-kernel-runtime-binary-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be monotonic")
            .as_nanos()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("temp root should be removable");
    }
    std::fs::create_dir_all(&root).expect("temp root should be creatable");
    AicoreLayout::new(root.join(".aicore"))
}

fn seed_foundation_binary(layout: &AicoreLayout) {
    seed_executable(
        &layout.bin_root.join(FOUNDATION_RUNTIME_BINARY_NAME),
        "#!/bin/sh\necho foundation-ok\n",
    );
}

fn seed_kernel_binary(layout: &AicoreLayout, script: &str) {
    seed_executable(&layout.bin_root.join(KERNEL_RUNTIME_BINARY_NAME), script);
}

fn seed_non_executable_kernel_binary(layout: &AicoreLayout) {
    let path = layout.bin_root.join(KERNEL_RUNTIME_BINARY_NAME);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("kernel binary parent should be creatable");
    }
    std::fs::write(&path, "#!/bin/sh\necho should-not-run\n")
        .expect("kernel binary should be writable");
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(&path)
            .expect("metadata should exist")
            .permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(&path, permissions).expect("permissions should be settable");
    }
}

fn seed_executable(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("binary parent should be creatable");
    }
    std::fs::write(path, content).expect("binary should be writable");
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(path)
            .expect("binary metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("binary permissions should be settable");
    }
}

fn fixture_success_script() -> &'static str {
    r#"#!/bin/sh
request=$(cat)
case "$request" in
  *super-secret-token*) echo "raw payload leaked" >&2 ;;
esac
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture","trace_id":"trace.default","operation":"runtime.status","status":"completed","route":{"component_id":"aicore","app_id":"aicore","capability_id":"runtime.status","contract_version":"kernel.app.v1"},"handler":{"kind":"kernel_runtime_binary","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"runtime.status","summary":"ok","fields":{"kernel_invocation_path":"binary","foundation_runtime_binary":"installed","kernel_runtime_binary":"installed","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","binary_health":"ok"}},"failure":{"stage":null,"reason":null}}}'
"#
}

#[test]
fn kernel_runtime_binary_client_reports_missing_binary() {
    let layout = temp_layout("missing-binary");
    seed_foundation_binary(&layout);

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

    assert!(!invocation.exit_success);
    assert_eq!(
        invocation.error.as_ref().map(|error| &error.kind),
        Some(&KernelRuntimeBinaryErrorKind::KernelBinaryMissing)
    );
    assert_eq!(
        invocation.payload["failure"]["stage"],
        "kernel_runtime_binary_missing"
    );
    assert_eq!(
        invocation.payload["runtime_binary"]["in_process_fallback"],
        false
    );
}

#[test]
fn kernel_runtime_binary_client_reports_non_executable_binary() {
    let layout = temp_layout("non-executable");
    seed_foundation_binary(&layout);
    seed_non_executable_kernel_binary(&layout);

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

    assert!(!invocation.exit_success);
    assert_eq!(
        invocation.error.as_ref().map(|error| &error.kind),
        Some(&KernelRuntimeBinaryErrorKind::KernelBinaryNotExecutable)
    );
    assert_eq!(
        invocation.payload["failure"]["stage"],
        "kernel_runtime_binary_not_executable"
    );
}

#[test]
fn kernel_runtime_binary_client_reports_spawn_failure() {
    let layout = temp_layout("spawn-failure");
    seed_foundation_binary(&layout);
    seed_kernel_binary(
        &layout,
        "#!/path/to/aicore/missing/interpreter\necho should-not-run\n",
    );

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

    assert!(!invocation.exit_success);
    assert_eq!(
        invocation.error.as_ref().map(|error| &error.kind),
        Some(&KernelRuntimeBinaryErrorKind::ProcessSpawnFailed)
    );
    assert_eq!(
        invocation.payload["failure"]["stage"],
        "kernel_runtime_process_spawn"
    );
}

#[test]
fn kernel_runtime_binary_client_reports_non_zero_exit() {
    let layout = temp_layout("non-zero-exit");
    seed_foundation_binary(&layout);
    seed_kernel_binary(
        &layout,
        "#!/bin/sh\ncat >/dev/null\necho broken >&2\nexit 7\n",
    );

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

    assert!(!invocation.exit_success);
    assert_eq!(
        invocation.error.as_ref().map(|error| &error.kind),
        Some(&KernelRuntimeBinaryErrorKind::NonZeroExit)
    );
    assert_eq!(
        invocation.payload["failure"]["stage"],
        "kernel_runtime_non_zero_exit"
    );
    assert_eq!(
        invocation.payload["handler"]["process_exit_code"],
        serde_json::Value::from(7)
    );
}

#[test]
fn kernel_runtime_binary_client_reports_invalid_jsonl_output() {
    let layout = temp_layout("invalid-jsonl-output");
    seed_foundation_binary(&layout);
    seed_kernel_binary(&layout, "#!/bin/sh\ncat >/dev/null\necho 'not json'\n");

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

    assert!(!invocation.exit_success);
    assert_eq!(
        invocation.error.as_ref().map(|error| &error.kind),
        Some(&KernelRuntimeBinaryErrorKind::InvalidJsonlOutput)
    );
    assert_eq!(
        invocation.payload["failure"]["stage"],
        "kernel_runtime_invalid_jsonl_output"
    );
}

#[test]
fn kernel_runtime_binary_client_reports_protocol_version_mismatch() {
    let layout = temp_layout("protocol-version-mismatch");
    seed_foundation_binary(&layout);
    seed_kernel_binary(
        &layout,
        r#"#!/bin/sh
cat >/dev/null
cat <<'JSON'
{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"wrong.version","contract_version":"kernel.runtime.v1","payload":{"status":"completed"}}
JSON
"#,
    );

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

    assert!(!invocation.exit_success);
    assert_eq!(
        invocation.error.as_ref().map(|error| &error.kind),
        Some(&KernelRuntimeBinaryErrorKind::ProtocolVersionMismatch)
    );
    assert_eq!(
        invocation.payload["failure"]["stage"],
        "kernel_runtime_protocol_version_mismatch"
    );
}

#[test]
fn kernel_runtime_binary_client_does_not_fallback_to_in_process() {
    let layout = temp_layout("no-fallback");
    seed_foundation_binary(&layout);

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

    assert!(!invocation.exit_success);
    assert_eq!(
        invocation.payload["runtime_binary"]["in_process_fallback"],
        false
    );
    assert_ne!(invocation.payload["status"], "completed");
}

#[test]
fn invocation_ledger_does_not_record_raw_runtime_protocol_payload() {
    let layout = temp_layout("raw-protocol-payload");
    seed_foundation_binary(&layout);
    seed_kernel_binary(&layout, fixture_success_script());

    let envelope = crate::KernelInvocationEnvelope::new(
        "global-main",
        "runtime.status",
        "runtime.status",
        crate::KernelPayload::Text("super-secret-token".to_string()),
    );
    let invocation = KernelRuntimeBinaryClient::new(layout.clone()).invoke_envelope(envelope);

    assert!(invocation.exit_success);
    let ledger_path = layout.kernel_state_root.join("invocation-ledger.jsonl");
    let ledger = std::fs::read_to_string(ledger_path).unwrap_or_default();
    assert!(!ledger.contains("super-secret-token"));
    assert!(!ledger.contains("raw payload leaked"));
}

#[test]
fn runtime_binary_failure_does_not_expose_secret_like_output() {
    let layout = temp_layout("redact-stderr");
    seed_foundation_binary(&layout);
    seed_kernel_binary(
        &layout,
        "#!/bin/sh\ncat >/dev/null\necho 'token=super-secret-token api_key=abc123' >&2\nexit 7\n",
    );

    let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");
    let failure = invocation.payload["failure"]["reason"]
        .as_str()
        .expect("failure reason should be a string");

    assert!(!failure.contains("super-secret-token"));
    assert!(!failure.contains("abc123"));
    assert!(failure.contains("[redacted:failure_reason]"));
}

#[allow(dead_code)]
fn _assert_paths_are_send_sync(_: PathBuf) {}
