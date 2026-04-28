use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    assert_has_json_event, assert_json_lines, run_cli_with_config_root, run_cli_with_env,
    seed_foundation_runtime_binary, seed_global_runtime_metadata,
    seed_kernel_runtime_binary_fixture, seed_provider_smoke_manifest, temp_root,
};

#[test]
fn provider_smoke_component_outputs_single_jsonl_result() {
    let root = temp_root("component-provider-smoke");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg("__component-provider-smoke-stdio")
        .env("AICORE_CONFIG_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("component handler should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(local_ipc_request("provider.smoke").as_bytes())
        .expect("request should be writable");
    let output = child
        .wait_with_output()
        .expect("component handler should finish");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://"));
    assert!(!stdout.contains("raw_sdk"));
    assert!(!stdout.contains("raw_provider"));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "provider.smoke");
    assert_eq!(value["fields"]["operation"], "provider.smoke");
    assert_eq!(value["fields"]["provider"], "dummy");
    assert_eq!(value["fields"]["live_call"], "false");
    assert_eq!(value["fields"]["sdk_live_call"], "false");
    assert_eq!(value["fields"]["network_used"], "false");
    assert_eq!(value["fields"]["kernel_invocation_path"], "binary");
}

#[test]
fn cli_kernel_invoke_readonly_provider_smoke_json_outputs_structured_fields() {
    let home = temp_root("kernel-readonly-provider-smoke-json");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_provider_smoke_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "provider.smoke"],
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

    assert_eq!(result["payload"]["operation"], "provider.smoke");
    assert_eq!(
        result["payload"]["route"]["component_id"],
        "aicore-provider-smoke"
    );
    assert_eq!(result["payload"]["handler"]["kind"], "local_process");
    assert_eq!(result["payload"]["handler"]["spawned_process"], true);
    assert_eq!(result["payload"]["result"]["kind"], "provider.smoke");
    assert_eq!(
        result["payload"]["result"]["fields"]["provider_invocation_path"],
        "smoke_readonly"
    );
    assert_eq!(result["payload"]["result"]["fields"]["live_call"], "false");
    assert_eq!(
        result["payload"]["result"]["fields"]["sdk_live_call"],
        "false"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["network_used"],
        "false"
    );
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://"));
    assert!(!stdout.contains("raw_sdk"));
    assert!(!stdout.contains("raw_provider"));
}

#[test]
fn cli_kernel_invoke_readonly_provider_smoke_has_no_in_process_fallback() {
    let home = temp_root("kernel-readonly-provider-smoke-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_provider_smoke_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "provider.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("handler executed：true"));
}

#[test]
fn direct_provider_smoke_remains_compatible() {
    let root = temp_root("direct-provider-smoke-compatible");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["provider", "smoke"], &root);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");

    assert!(stdout.contains("Provider Smoke"));
    assert!(stdout.contains("auth_ref"));
    assert!(stdout.contains("provider response"));
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://"));
}

fn local_ipc_request(operation: &str) -> String {
    serde_json::json!({
        "schema_version": "aicore.local_ipc.invocation.v1",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.local_ipc.stdio_jsonl.v1",
        "invocation_id": format!("invoke.test.{operation}"),
        "trace_id": "trace.test",
        "instance_id": "global-main",
        "operation": operation,
        "route": {
            "component_id": "aicore-provider-smoke",
            "app_id": "aicore-cli",
            "capability_id": operation,
            "contract_version": "kernel.app.v1"
        }
    })
    .to_string()
        + "\n"
}
