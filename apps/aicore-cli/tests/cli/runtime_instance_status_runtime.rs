use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    assert_has_json_event, assert_json_lines, run_cli_with_env, seed_foundation_runtime_binary,
    seed_global_runtime_metadata, seed_kernel_runtime_binary_fixture,
    seed_runtime_instance_status_manifests, temp_root,
};

#[test]
fn runtime_smoke_component_outputs_single_jsonl_result() {
    let output = run_component_handler("__component-runtime-smoke-stdio", "runtime.smoke");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "runtime.smoke");
    assert_eq!(value["fields"]["operation"], "runtime.smoke");
    assert_eq!(value["fields"]["status"], "ok");
    assert!(value["fields"]["diagnostics"].is_string());
}

#[test]
fn instance_list_component_outputs_single_jsonl_result() {
    let output = run_component_handler("__component-instance-list-stdio", "instance.list");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "instance.list");
    assert_eq!(value["fields"]["operation"], "instance.list");
    assert_eq!(value["fields"]["instance_count"], "1");
    assert!(
        value["fields"]["instances"]
            .as_str()
            .unwrap()
            .contains("global-main")
    );
}

#[test]
fn cli_status_component_outputs_single_jsonl_result() {
    let output = run_component_handler("__component-status-stdio", "cli.status");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "cli.status");
    assert_eq!(value["fields"]["operation"], "cli.status");
    assert_eq!(value["fields"]["app"], "aicore-cli");
    assert_eq!(value["fields"]["kernel_invocation_path"], "binary");
}

#[test]
fn cli_kernel_invoke_readonly_runtime_instance_status_json_outputs_structured_fields() {
    for (operation, result_kind, field_name) in [
        ("runtime.smoke", "runtime.smoke", "status"),
        ("instance.list", "instance.list", "instance_count"),
        ("cli.status", "cli.status", "app"),
    ] {
        let home = temp_root(&format!(
            "kernel-readonly-{}-json",
            operation.replace('.', "-")
        ));
        seed_global_runtime_metadata(&home);
        seed_foundation_runtime_binary(&home);
        seed_kernel_runtime_binary_fixture(&home);
        seed_runtime_instance_status_manifests(&home);

        let output = run_cli_with_env(
            &["kernel", "invoke-readonly", operation],
            &[
                ("HOME", home.to_str().expect("home path should be utf-8")),
                ("AICORE_TERMINAL", "json"),
            ],
        );

        assert!(output.status.success(), "{operation} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let events = assert_json_lines(&stdout);
        assert_has_json_event(&events, "kernel.invocation.result");
        let result = events
            .iter()
            .find(|event| event["event"] == "kernel.invocation.result")
            .expect("result event should exist");

        assert_eq!(result["payload"]["operation"], operation);
        assert_eq!(result["payload"]["result"]["kind"], result_kind);
        assert!(result["payload"]["result"]["fields"][field_name].is_string());
        assert_eq!(
            result["payload"]["result"]["fields"]["kernel_invocation_path"],
            "binary"
        );
    }
}

#[test]
fn cli_kernel_invoke_readonly_runtime_instance_status_has_no_in_process_fallback() {
    let home = temp_root("kernel-readonly-runtime-instance-status-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_runtime_instance_status_manifests(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("handler executed：true"));
}

#[test]
fn direct_runtime_instance_status_commands_remain_compatible() {
    let runtime = run_cli_with_env(&["runtime", "smoke"], &[]);
    assert!(runtime.status.success());
    let runtime_stdout = String::from_utf8(runtime.stdout).expect("stdout should be utf-8");
    assert!(runtime_stdout.contains("Runtime Smoke"));
    assert!(runtime_stdout.contains("CLI 场景"));

    let instances = run_cli_with_env(&["instance", "list"], &[]);
    assert!(instances.status.success());
    let instances_stdout = String::from_utf8(instances.stdout).expect("stdout should be utf-8");
    assert!(instances_stdout.contains("实例列表"));
    assert!(instances_stdout.contains("global-main"));

    let status = run_cli_with_env(&["status"], &[]);
    assert!(status.status.success());
    let status_stdout = String::from_utf8(status.stdout).expect("stdout should be utf-8");
    assert!(status_stdout.contains("AICore CLI"));
    assert!(status_stdout.contains("主实例"));
}

fn run_component_handler(command: &str, operation: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("component handler should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(local_ipc_request(operation).as_bytes())
        .expect("request should be writable");
    child
        .wait_with_output()
        .expect("component handler should finish")
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
            "component_id": format!("aicore-{}", operation.replace('.', "-")),
            "app_id": "aicore-cli",
            "capability_id": operation,
            "contract_version": "kernel.app.v1"
        }
    })
    .to_string()
        + "\n"
}
