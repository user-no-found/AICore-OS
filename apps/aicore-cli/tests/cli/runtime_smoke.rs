use super::support::*;

#[test]
fn renders_runtime_smoke_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["runtime", "smoke"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("runtime.smoke"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn cli_runtime_smoke_rich_uses_terminal_document() {
    let output = run_cli_with_env(&["runtime", "smoke"], &[("AICORE_TERMINAL", "rich")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
}

#[test]
fn cli_runtime_smoke_json_outputs_valid_json() {
    let output = run_cli_with_env(&["runtime", "smoke"], &[("AICORE_TERMINAL", "json")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "kernel.invocation.result")
    );
    assert!(stdout.contains("runtime.smoke"));
    assert!(!stdout.contains("Runtime Smoke\n"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn renders_runtime_smoke_local_direct_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["runtime", "smoke", "--local"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Runtime Smoke（local direct）"));
    assert!(stdout.contains("CLI 场景"));
    assert!(stdout.contains("execution_path：local_direct"));
    assert!(stdout.contains("kernel_invocation_path：not_used"));
    assert!(stdout.contains("ledger_appended：false"));
    assert!(
        stdout
            .contains("本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger")
    );
}

#[test]
fn cli_runtime_smoke_local_direct_json_outputs_direct_command_result() {
    let output = run_cli_with_env(
        &["runtime", "smoke", "--local"],
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
        .expect("direct.command.result event should exist");
    assert_eq!(event["operation"], "runtime.smoke");
    assert_eq!(event["success"], true);
    assert_eq!(event["execution_path"], "local_direct");
    assert_eq!(event["kernel_invocation_path"], "not_used");
    assert_eq!(event["ledger_appended"], false);
    assert!(event["fields"]["operation"].is_string());
}
