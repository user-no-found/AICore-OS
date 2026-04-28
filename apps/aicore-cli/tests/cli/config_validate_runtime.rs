use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    assert_has_json_event, assert_json_lines, run_cli_with_env, seed_config_validate_manifest,
    seed_foundation_runtime_binary, seed_global_runtime_metadata,
    seed_kernel_runtime_binary_fixture, temp_root,
};

#[test]
fn config_validate_component_reads_stdio_jsonl_request() {
    let root = temp_root("component-config-validate-stdio");
    let init_output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");
    assert!(init_output.status.success());

    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg("__component-config-validate-stdio")
        .env("AICORE_CONFIG_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("component handler should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(local_ipc_request().as_bytes())
        .expect("request should be writable");
    let output = child
        .wait_with_output()
        .expect("component handler should finish");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");

    assert_eq!(value["schema_version"], "aicore.local_ipc.result.v1");
    assert_eq!(value["protocol"], "stdio_jsonl");
    assert_eq!(value["invocation_id"], "invoke.test.config.validate");
    assert_eq!(value["status"], "completed");
    assert_eq!(value["result_kind"], "config.validate");
    assert_eq!(value["fields"]["operation"], "config.validate");
    assert_eq!(value["fields"]["valid"], "true");
    assert_eq!(value["fields"]["error_count"], "0");
    assert!(
        value["fields"]["checked_files"]
            .as_str()
            .expect("checked files should be string")
            .contains("auth.toml")
    );
}

#[test]
fn config_validate_component_does_not_output_human_panel_or_secret_ref() {
    let root = temp_root("component-config-validate-no-secret");
    let init_output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");
    assert!(init_output.status.success());

    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg("__component-config-validate-stdio")
        .env("AICORE_CONFIG_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("component handler should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(local_ipc_request().as_bytes())
        .expect("request should be writable");
    let output = child
        .wait_with_output()
        .expect("component handler should finish");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("配置校验："));
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("credential_lease_ref"));
}

#[test]
fn cli_kernel_invoke_readonly_config_validate_outputs_chinese_summary() {
    let home = temp_root("kernel-readonly-config-validate-human");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_config_validate_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "config.validate"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("operation：config.validate"));
    assert!(stdout.contains("component：aicore-config-validate"));
    assert!(stdout.contains("invocation mode：local_process"));
    assert!(stdout.contains("transport：stdio_jsonl"));
    assert!(stdout.contains("result kind：config.validate"));
    assert!(stdout.contains("result.valid：true"));
    assert!(stdout.contains("in-process fallback：false"));
}

#[test]
fn cli_kernel_invoke_readonly_config_validate_json_outputs_structured_fields() {
    let home = temp_root("kernel-readonly-config-validate-json");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_config_validate_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "config.validate"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "kernel.invocation.result");
    let result = events
        .iter()
        .find(|event| event["event"] == "kernel.invocation.result")
        .expect("result event should exist");

    assert_eq!(result["payload"]["operation"], "config.validate");
    assert_eq!(result["payload"]["result"]["kind"], "config.validate");
    assert_eq!(result["payload"]["result"]["fields"]["valid"], "true");
    assert_eq!(
        result["payload"]["result"]["fields"]["kernel_invocation_path"],
        "binary"
    );
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn cli_kernel_invoke_readonly_config_validate_has_no_in_process_fallback() {
    let home = temp_root("kernel-readonly-config-validate-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_config_validate_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "config.validate"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("handler executed：true"));
}

fn local_ipc_request() -> String {
    serde_json::json!({
        "schema_version": "aicore.local_ipc.invocation.v1",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.local_ipc.stdio_jsonl.v1",
        "invocation_id": "invoke.test.config.validate",
        "trace_id": "trace.test",
        "instance_id": "global-main",
        "operation": "config.validate",
        "route": {
            "component_id": "aicore-config-validate",
            "app_id": "aicore-cli",
            "capability_id": "config.validate",
            "contract_version": "kernel.app.v1"
        }
    })
    .to_string()
        + "\n"
}
