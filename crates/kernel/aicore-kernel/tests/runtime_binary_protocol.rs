use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aicore-kernel-runtime-protocol-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("temp root should be removable");
    }
    std::fs::create_dir_all(&root).expect("temp root should be creatable");
    root
}

fn seed_runtime_layout(home: &Path) {
    let root = home.join(".aicore");
    let foundation = root.join("runtime").join("foundation");
    let kernel = root.join("runtime").join("kernel");
    let manifests = root.join("share").join("manifests");
    let bin = root.join("bin");

    std::fs::create_dir_all(&foundation).expect("foundation metadata dir should be creatable");
    std::fs::create_dir_all(&kernel).expect("kernel metadata dir should be creatable");
    std::fs::create_dir_all(&manifests).expect("manifest dir should be creatable");
    std::fs::create_dir_all(&bin).expect("bin dir should be creatable");

    std::fs::write(
        foundation.join("install.toml"),
        format!(
            "layer = \"foundation\"\nruntime_binary_path = \"{}\"\nruntime_protocol = \"stdio_jsonl\"\ncontract_version = \"kernel.runtime.v1\"\nhealth = \"installed\"\n",
            bin.join("aicore-foundation").display()
        ),
    )
    .expect("foundation metadata should be writable");
    std::fs::write(
        kernel.join("install.toml"),
        format!(
            "layer = \"kernel\"\nruntime_binary_path = \"{}\"\nruntime_protocol = \"stdio_jsonl\"\ncontract_version = \"kernel.runtime.v1\"\nhealth = \"installed\"\n",
            bin.join("aicore-kernel").display()
        ),
    )
    .expect("kernel metadata should be writable");
    std::fs::write(
        kernel.join("version.toml"),
        "contract_version = \"kernel.runtime.v1\"\n",
    )
    .expect("kernel version should be writable");
    std::fs::write(
        manifests.join("aicore.toml"),
        format!(
            "component_id = \"aicore\"\napp_id = \"aicore\"\nkind = \"app\"\nentrypoint = \"{}\"\ncontract_version = \"kernel.app.v1\"\n\n[[capabilities]]\nid = \"runtime.status\"\noperation = \"runtime.status\"\nvisibility = \"user\"\n",
            bin.join("aicore").display()
        ),
    )
    .expect("manifest should be writable");
    seed_executable(
        &bin.join("aicore-foundation"),
        "#!/bin/sh\necho foundation-runtime-ok\n",
    );
}

fn seed_component_process_manifest(home: &Path, script: &str) {
    let root = home.join(".aicore");
    let bin = root.join("bin");
    let manifests = root.join("share").join("manifests");
    let component = bin.join("component-process-smoke");
    seed_executable(&component, script);
    std::fs::write(
        manifests.join("aicore-component-smoke.toml"),
        format!(
            "component_id = \"aicore-component-smoke\"\napp_id = \"aicore-component-smoke\"\nkind = \"app\"\nentrypoint = \"{}\"\ninvocation_mode = \"local_process\"\ntransport = \"stdio_jsonl\"\ncontract_version = \"kernel.app.v1\"\n\n[[capabilities]]\nid = \"component.process.smoke\"\noperation = \"component.process.smoke\"\nvisibility = \"diagnostic\"\n",
            component.display()
        ),
    )
    .expect("process component manifest should be writable");
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
        std::fs::set_permissions(path, permissions).expect("binary should be executable");
    }
}

fn invoke_kernel_binary(home: &Path, request: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-kernel"))
        .arg("--invoke-stdio-jsonl")
        .env("HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("aicore-kernel should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(request.as_bytes())
        .expect("request should be writable");
    child
        .wait_with_output()
        .expect("aicore-kernel output should be readable")
}

fn valid_request() -> String {
    valid_request_for("runtime.status")
}

fn valid_request_for(operation: &str) -> String {
    serde_json::json!({
        "schema_version": "aicore.kernel.runtime_binary.request.v1",
        "request_id": "request.test",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.kernel.runtime_binary.stdio_jsonl.v1",
        "contract_version": "kernel.runtime.v1",
        "invocation_id": "invoke.test",
        "trace_id": "trace.test",
        "instance_id": "global-main",
        "capability": operation,
        "operation": operation,
        "payload": "empty",
        "ledger_path": "unused"
    })
    .to_string()
        + "\n"
}

fn first_event(stdout: &[u8]) -> serde_json::Value {
    let stdout = String::from_utf8(stdout.to_vec()).expect("stdout should be utf-8");
    let line = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .expect("stdout should contain a JSON line");
    serde_json::from_str(line).expect("stdout line should be valid JSON")
}

fn ledger_lines(home: &Path) -> Vec<serde_json::Value> {
    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = std::fs::read_to_string(ledger_path).expect("ledger should exist");
    ledger
        .lines()
        .map(|line| serde_json::from_str(line).expect("ledger line should be valid JSON"))
        .collect()
}

#[test]
fn kernel_runtime_binary_status_reports_protocol_contract() {
    let home = temp_home("status");
    seed_runtime_layout(&home);

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-kernel"))
        .arg("--status")
        .env("HOME", &home)
        .output()
        .expect("aicore-kernel status should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("protocol: stdio_jsonl"));
    assert!(stdout.contains("protocol version: aicore.kernel.runtime_binary.stdio_jsonl.v1"));
    assert!(stdout.contains("contract version: kernel.runtime.v1"));
    assert!(stdout.contains("foundation runtime health: ok"));
}

#[test]
fn kernel_runtime_binary_invoke_accepts_valid_jsonl_request() {
    let home = temp_home("valid-request");
    seed_runtime_layout(&home);

    let output = invoke_kernel_binary(&home, &valid_request());

    assert!(output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(event["event"], "kernel.invocation.result");
    assert_eq!(event["protocol"], "stdio_jsonl");
    assert_eq!(
        event["protocol_version"],
        "aicore.kernel.runtime_binary.stdio_jsonl.v1"
    );
    assert_eq!(event["contract_version"], "kernel.runtime.v1");
    assert_eq!(event["payload"]["status"], "completed");
    assert_eq!(
        event["payload"]["result"]["fields"]["protocol_version"],
        "aicore.kernel.runtime_binary.stdio_jsonl.v1"
    );
}

#[test]
fn kernel_runtime_binary_spawns_component_process_from_manifest_entrypoint() {
    let home = temp_home("component-process-success");
    seed_runtime_layout(&home);
    seed_component_process_manifest(
        &home,
        "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '{\"result_kind\":\"component.process.smoke\",\"summary\":\"process smoke ok\",\"fields\":{\"operation\":\"component.process.smoke\",\"ipc\":\"stdio_jsonl\",\"component_process\":\"ok\"}}'\n",
    );

    let output = invoke_kernel_binary(&home, &valid_request_for("component.process.smoke"));

    assert!(output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(event["payload"]["status"], "completed");
    assert_eq!(
        event["payload"]["route"]["component_id"],
        "aicore-component-smoke"
    );
    assert_eq!(event["payload"]["handler"]["kind"], "local_process");
    assert_eq!(event["payload"]["handler"]["spawned_process"], true);
    assert_eq!(event["payload"]["handler"]["transport"], "stdio_jsonl");
    assert_eq!(
        event["payload"]["result"]["kind"],
        "component.process.smoke"
    );
    assert_eq!(event["payload"]["result"]["fields"]["ipc"], "stdio_jsonl");

    let ledger = ledger_lines(&home);
    assert_eq!(ledger.len(), 5);
    assert!(ledger.iter().all(|record| {
        record["invocation_id"] == "invoke.test" && !record.to_string().contains("process smoke ok")
    }));
    assert_eq!(ledger[2]["handler_kind"], "local_process");
    assert_eq!(ledger[2]["spawned_process"], true);
    assert_eq!(ledger[2]["transport"], "stdio_jsonl");
}

#[test]
fn kernel_runtime_binary_component_process_non_zero_exit_is_structured() {
    let home = temp_home("component-process-nonzero");
    seed_runtime_layout(&home);
    seed_component_process_manifest(
        &home,
        "#!/bin/sh\ncat >/dev/null\necho 'token=super-secret-token api_key=abc123' >&2\nexit 7\n",
    );

    let output = invoke_kernel_binary(&home, &valid_request_for("component.process.smoke"));

    assert!(!output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(event["payload"]["status"], "failed");
    assert_eq!(event["payload"]["failure"]["stage"], "process_exit");
    assert_eq!(event["payload"]["handler"]["spawned_process"], true);
    assert_eq!(event["payload"]["handler"]["process_exit_code"], 7);
    let payload = event["payload"].to_string();
    assert!(!payload.contains("super-secret-token"));
    assert!(!payload.contains("api_key=abc123"));
}

#[test]
fn kernel_runtime_binary_component_process_invalid_jsonl_is_structured() {
    let home = temp_home("component-process-invalid-jsonl");
    seed_runtime_layout(&home);
    seed_component_process_manifest(
        &home,
        "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' 'not-json'\n",
    );

    let output = invoke_kernel_binary(&home, &valid_request_for("component.process.smoke"));

    assert!(!output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(event["payload"]["status"], "failed");
    assert_eq!(event["payload"]["failure"]["stage"], "ipc_read");
    assert_eq!(event["payload"]["handler"]["spawned_process"], true);
    assert_eq!(event["payload"]["handler"]["transport"], "stdio_jsonl");
}

#[test]
fn kernel_runtime_binary_invoke_rejects_malformed_jsonl() {
    let home = temp_home("malformed-jsonl");
    seed_runtime_layout(&home);

    let output = invoke_kernel_binary(&home, "{not json}\n");

    assert!(!output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(event["event"], "kernel.invocation.result");
    assert_eq!(
        event["payload"]["failure"]["stage"],
        "kernel_runtime_malformed_jsonl"
    );
    assert_eq!(event["payload"]["ledger"]["appended"], false);
}

#[test]
fn kernel_runtime_binary_invoke_rejects_protocol_version_mismatch() {
    let home = temp_home("protocol-version-mismatch");
    seed_runtime_layout(&home);
    let mut request: serde_json::Value =
        serde_json::from_str(valid_request().trim()).expect("request should be json");
    request["protocol_version"] = serde_json::Value::String("wrong.version".to_string());

    let output = invoke_kernel_binary(&home, &(request.to_string() + "\n"));

    assert!(!output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(
        event["payload"]["failure"]["stage"],
        "kernel_runtime_protocol_version_mismatch"
    );
    assert_eq!(event["payload"]["ledger"]["appended"], false);
}

#[test]
fn kernel_runtime_binary_invoke_returns_structured_failure_on_missing_foundation() {
    let home = temp_home("missing-foundation");
    seed_runtime_layout(&home);
    std::fs::remove_file(home.join(".aicore").join("bin").join("aicore-foundation"))
        .expect("foundation binary should be removable");

    let output = invoke_kernel_binary(&home, &valid_request());

    assert!(!output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(
        event["payload"]["failure"]["stage"],
        "foundation_runtime_binary_missing"
    );
    assert_eq!(
        event["payload"]["runtime_binary"]["foundation_health"],
        "missing"
    );
    assert_eq!(event["payload"]["ledger"]["appended"], false);
}

#[test]
#[cfg(unix)]
fn kernel_runtime_binary_invoke_returns_structured_failure_on_unhealthy_foundation() {
    let home = temp_home("unhealthy-foundation");
    seed_runtime_layout(&home);
    let foundation_binary = home.join(".aicore").join("bin").join("aicore-foundation");
    let mut permissions = std::fs::metadata(&foundation_binary)
        .expect("foundation binary metadata should exist")
        .permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&foundation_binary, permissions)
        .expect("foundation permissions should be writable");

    let output = invoke_kernel_binary(&home, &valid_request());

    assert!(!output.status.success());
    let event = first_event(&output.stdout);
    assert_eq!(
        event["payload"]["failure"]["stage"],
        "foundation_runtime_binary_not_executable"
    );
    assert_eq!(
        event["payload"]["runtime_binary"]["foundation_health"],
        "not_executable"
    );
    assert_eq!(event["payload"]["ledger"]["appended"], false);
}
