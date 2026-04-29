use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    assert_has_json_event, assert_json_lines, run_cli_with_config_root,
    run_cli_with_config_root_and_env, run_cli_with_env, seed_agent_smoke_manifests,
    seed_foundation_runtime_binary, seed_global_runtime_metadata,
    seed_kernel_runtime_binary_fixture, temp_root,
};

#[test]
fn agent_smoke_component_outputs_single_jsonl_result() {
    let root = temp_root("component-agent-smoke");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_component("__component-agent-smoke-stdio", "agent.smoke", &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains("hello from hidden prompt"));
    assert!(!stdout.contains("CURRENT USER REQUEST"));
    assert!(!stdout.contains("raw_provider"));
    assert!(!stdout.contains("secret_ref"));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "agent.smoke");
    assert_eq!(value["fields"]["operation"], "agent.smoke");
    assert_eq!(value["fields"]["real_provider"], "false");
    assert_eq!(value["fields"]["tool_calling"], "false");
    assert_eq!(value["fields"]["streaming"], "false");
    assert_eq!(value["fields"]["kernel_invocation_path"], "binary");
}

#[test]
fn agent_session_smoke_component_outputs_single_jsonl_result() {
    let root = temp_root("component-agent-session-smoke");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_component(
        "__component-agent-session-smoke-stdio",
        "agent.session_smoke",
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.contains("first hidden prompt"));
    assert!(!stdout.contains("second hidden prompt"));
    assert!(!stdout.contains("CURRENT USER REQUEST"));
    assert!(!stdout.contains("raw memory"));

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["result_kind"], "agent.session_smoke");
    assert_eq!(value["fields"]["operation"], "agent.session_smoke");
    assert_eq!(value["fields"]["turn_count"], "2");
    assert_eq!(value["fields"]["real_provider"], "false");
    assert_eq!(value["fields"]["tool_calling"], "false");
    assert_eq!(value["fields"]["streaming"], "false");
}

#[test]
fn cli_kernel_invoke_readonly_agent_smoke_json_outputs_structured_fields() {
    let home = runtime_home("kernel-readonly-agent-smoke-json");

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "agent.smoke", "hello"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let result = kernel_result_event(&stdout);

    assert_eq!(result["payload"]["operation"], "agent.smoke");
    assert_eq!(
        result["payload"]["route"]["component_id"],
        "aicore-agent-smoke"
    );
    assert_eq!(result["payload"]["handler"]["kind"], "local_process");
    assert_eq!(result["payload"]["handler"]["spawned_process"], true);
    assert_eq!(result["payload"]["result"]["kind"], "agent.smoke");
    assert_eq!(
        result["payload"]["result"]["fields"]["real_provider"],
        "false"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["tool_calling"],
        "false"
    );
    assert_eq!(result["payload"]["result"]["fields"]["streaming"], "false");
    assert!(!stdout.contains("hello"));
    assert!(!stdout.contains("CURRENT USER REQUEST"));
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn cli_kernel_invoke_readonly_agent_session_smoke_json_outputs_structured_fields() {
    let home = runtime_home("kernel-readonly-agent-session-json");

    let output = run_cli_with_env(
        &[
            "kernel",
            "invoke-readonly",
            "agent.session_smoke",
            "session-secret-input-one",
            "session-secret-input-two",
        ],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let result = kernel_result_event(&stdout);

    assert_eq!(result["payload"]["operation"], "agent.session_smoke");
    assert_eq!(
        result["payload"]["route"]["component_id"],
        "aicore-agent-session-smoke"
    );
    assert_eq!(result["payload"]["result"]["kind"], "agent.session_smoke");
    assert_eq!(result["payload"]["result"]["fields"]["turn_count"], "2");
    assert_eq!(
        result["payload"]["result"]["fields"]["completed_all_inputs"],
        "true"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["real_provider"],
        "false"
    );
    assert_eq!(result["payload"]["result"]["fields"]["streaming"], "false");
    assert!(!stdout.contains("session-secret-input-one"));
    assert!(!stdout.contains("session-secret-input-two"));
    assert!(!stdout.contains("raw memory"));
}

#[test]
fn cli_kernel_invoke_readonly_agent_smoke_has_no_in_process_fallback() {
    let home = temp_root("kernel-readonly-agent-smoke-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_agent_smoke_manifests(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "agent.smoke", "hello"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("handler executed：true"));
}

#[test]
fn direct_agent_smoke_commands_remain_compatible() {
    let root = temp_root("direct-agent-smoke-compatible");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let smoke = run_cli_with_config_root(&["agent", "smoke", "--local", "hello"], &root);
    assert!(smoke.status.success());
    let smoke_stdout = String::from_utf8(smoke.stdout).expect("stdout should be utf-8");
    assert!(smoke_stdout.contains("Agent Loop（local direct）"));
    assert!(smoke_stdout.contains("provider invoked"));
    assert!(!smoke_stdout.contains("CURRENT USER REQUEST"));

    let session = run_cli_with_config_root(
        &["agent", "session-smoke", "--local", "first", "second"],
        &root,
    );
    assert!(session.status.success());
    let session_stdout = String::from_utf8(session.stdout).expect("stdout should be utf-8");
    assert!(session_stdout.contains("Agent Session（local direct）"));
    assert!(session_stdout.contains("turns：2"));
    assert!(!session_stdout.contains("CURRENT USER REQUEST"));
}

#[test]
fn agent_smoke_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["agent", "smoke", "hello"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("agent.smoke"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn agent_session_smoke_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["agent", "session-smoke", "first", "second"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("agent.session_smoke"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn cli_agent_smoke_local_rich_uses_terminal_panel() {
    let root = temp_root("agent-smoke-local-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "smoke", "--local", "hello"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Agent Loop（local direct）"));
    assert!(stdout.contains("outcome：completed"));
    assert!(!stdout.contains("SYSTEM:"));
}

#[test]
fn cli_agent_smoke_local_json_outputs_valid_json() {
    let root = temp_root("agent-smoke-local-json-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "smoke", "--local", "hello"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "agent.smoke");
    assert_eq!(event["fields"]["real_provider"], "false");
    assert_eq!(event["fields"]["tool_calling"], "false");
    assert_eq!(event["fields"]["streaming"], "false");
    assert!(!stdout.contains("Agent Loop："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_agent_session_smoke_local_rich_uses_terminal_panel() {
    let root = temp_root("agent-session-smoke-local-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "session-smoke", "--local", "first", "second"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Agent Session（local direct）"));
    assert!(stdout.contains("turn 1 outcome：completed"));
    assert!(stdout.contains("turn 2 outcome：completed"));
    assert!(!stdout.contains("SYSTEM:"));
}

#[test]
fn cli_agent_session_smoke_local_json_outputs_valid_json() {
    let root = temp_root("agent-session-smoke-local-json-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "session-smoke", "--local", "first", "second"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "agent.session_smoke");
    assert_eq!(event["fields"]["turn_count"], "2");
    assert_eq!(event["fields"]["real_provider"], "false");
    assert_eq!(event["fields"]["tool_calling"], "false");
    assert_eq!(event["fields"]["streaming"], "false");
    assert!(!stdout.contains("Agent Session："));
    assert!(!stdout.contains("\u{1b}["));
}

fn runtime_home(name: &str) -> std::path::PathBuf {
    let home = temp_root(name);
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_agent_smoke_manifests(&home);
    home
}

fn kernel_result_event(stdout: &str) -> serde_json::Value {
    let events = assert_json_lines(stdout);
    assert_has_json_event(&events, "kernel.invocation.result");
    events
        .into_iter()
        .find(|event| event["event"] == "kernel.invocation.result")
        .expect("result event should exist")
}

fn run_component(command: &str, operation: &str, root: &std::path::Path) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg(command)
        .env("AICORE_CONFIG_ROOT", root)
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
    let payload = match operation {
        "agent.session_smoke" => serde_json::json!({
            "first": "first hidden prompt",
            "second": "second hidden prompt"
        }),
        _ => serde_json::json!({ "content": "hello from hidden prompt" }),
    };
    serde_json::json!({
        "schema_version": "aicore.local_ipc.invocation.v1",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.local_ipc.stdio_jsonl.v1",
        "invocation_id": format!("invoke.test.{operation}"),
        "trace_id": "trace.test",
        "instance_id": "global-main",
        "operation": operation,
        "payload": payload,
        "route": {
            "component_id": format!("aicore-{operation}"),
            "app_id": "aicore-cli",
            "capability_id": operation,
            "contract_version": "kernel.app.v1"
        }
    })
    .to_string()
        + "\n"
}
