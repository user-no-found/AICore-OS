use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    assert_has_json_event, assert_json_lines, run_cli_with_env, seed_auth_model_service_manifests,
    seed_foundation_runtime_binary, seed_global_runtime_metadata,
    seed_kernel_runtime_binary_fixture, temp_root,
};

#[test]
fn auth_list_component_outputs_single_jsonl_result() {
    let output = run_component_handler("__component-auth-list-stdio", "auth.list");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://"));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "auth.list");
    assert_eq!(value["fields"]["operation"], "auth.list");
    assert_eq!(value["fields"]["auth_count"], "4");
    assert!(
        value["fields"]["entries"]
            .as_str()
            .unwrap()
            .contains("configured")
    );
}

#[test]
fn model_show_component_outputs_single_jsonl_result() {
    let output = run_component_handler("__component-model-show-stdio", "model.show");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "model.show");
    assert_eq!(value["fields"]["operation"], "model.show");
    assert_eq!(value["fields"]["primary_model"], "dummy/default-chat");
    assert_eq!(value["fields"]["primary_auth_ref"], "auth.dummy.main");
}

#[test]
fn service_list_component_outputs_single_jsonl_result() {
    let output = run_component_handler("__component-service-list-stdio", "service.list");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "service.list");
    assert_eq!(value["fields"]["operation"], "service.list");
    assert_eq!(value["fields"]["service_count"], "3");
    assert!(
        value["fields"]["services"]
            .as_str()
            .unwrap()
            .contains("search")
    );
}

#[test]
fn cli_kernel_invoke_readonly_auth_model_service_json_outputs_structured_fields() {
    for (operation, result_kind, field_name) in [
        ("auth.list", "auth.list", "auth_count"),
        ("model.show", "model.show", "primary_model"),
        ("service.list", "service.list", "service_count"),
    ] {
        let home = temp_root(&format!(
            "kernel-readonly-{}-json",
            operation.replace('.', "-")
        ));
        seed_global_runtime_metadata(&home);
        seed_foundation_runtime_binary(&home);
        seed_kernel_runtime_binary_fixture(&home);
        seed_auth_model_service_manifests(&home);

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
        assert!(!stdout.contains("secret_ref"));
        assert!(!stdout.contains("secret://"));
    }
}

#[test]
fn cli_kernel_invoke_readonly_auth_model_service_has_no_in_process_fallback() {
    let home = temp_root("kernel-readonly-auth-model-service-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_auth_model_service_manifests(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "auth.list"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("handler executed：true"));
}

fn run_component_handler(command: &str, operation: &str) -> std::process::Output {
    let root = temp_root(&format!("component-{}", operation.replace('.', "-")));
    let init_output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");
    assert!(init_output.status.success());

    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg(command)
        .env("AICORE_CONFIG_ROOT", &root)
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
