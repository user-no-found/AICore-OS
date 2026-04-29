use super::support::*;

#[test]
fn renders_status_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg("status")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("cli.status"));
    assert!(stdout.contains("foundation installed"));
    assert!(stdout.contains("kernel installed"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn cli_status_uses_terminal_document_in_rich_mode() {
    let output = run_cli_with_env(&["status"], &[("AICORE_TERMINAL", "rich")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("foundation installed"));
    assert!(stdout.contains("kernel installed"));
}

#[test]
fn cli_status_json_outputs_valid_json() {
    let output = run_cli_with_env(&["status"], &[("AICORE_TERMINAL", "json")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "kernel.invocation.result")
    );
    assert!(stdout.contains("cli.status"));
    assert!(!stdout.contains("AICore CLI\n"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn renders_status_local_direct_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["status", "--local"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("AICore CLI (local direct)"));
    assert!(stdout.contains("主实例：global-main"));
    assert!(stdout.contains("组件数量："));
    assert!(stdout.contains("实例数量："));
    assert!(stdout.contains("Runtime：global-main/main"));
    assert!(stdout.contains("execution_path：local_direct"));
    assert!(stdout.contains("kernel_invocation_path：not_used"));
    assert!(stdout.contains("ledger_appended：false"));
    assert!(
        stdout
            .contains("本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger")
    );
}

#[test]
fn cli_status_local_direct_json_outputs_direct_command_result() {
    let output = run_cli_with_env(&["status", "--local"], &[("AICORE_TERMINAL", "json")]);

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
    assert_eq!(event["operation"], "cli.status");
    assert_eq!(event["success"], true);
    assert_eq!(event["execution_path"], "local_direct");
    assert_eq!(event["kernel_invocation_path"], "not_used");
    assert_eq!(event["ledger_appended"], false);
    assert!(event["fields"]["main_instance"].is_string());
}

#[test]
fn renders_instance_list_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["instance", "list"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("instance.list"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn cli_instance_list_rich_uses_terminal_document() {
    let output = run_cli_with_env(&["instance", "list"], &[("AICORE_TERMINAL", "rich")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("instance.list"));
}

#[test]
fn cli_instance_list_plain_has_no_ansi() {
    let output = run_cli_with_env(&["instance", "list"], &[("AICORE_TERMINAL", "plain")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("instance.list"));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_instance_list_json_outputs_valid_json() {
    let output = run_cli_with_env(&["instance", "list"], &[("AICORE_TERMINAL", "json")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "kernel.invocation.result")
    );
    assert!(stdout.contains("instance.list"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn renders_instance_list_local_direct_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["instance", "list", "--local"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("实例列表（local direct）"));
    assert!(stdout.contains("global-main"));
    assert!(stdout.contains("global_main"));
    assert!(stdout.contains("execution_path：local_direct"));
    assert!(stdout.contains("kernel_invocation_path：not_used"));
    assert!(stdout.contains("ledger_appended：false"));
    assert!(
        stdout
            .contains("本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger")
    );
}

#[test]
fn cli_instance_list_local_direct_json_outputs_direct_command_result() {
    let output = run_cli_with_env(
        &["instance", "list", "--local"],
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
    assert_eq!(event["operation"], "instance.list");
    assert_eq!(event["success"], true);
    assert_eq!(event["execution_path"], "local_direct");
    assert_eq!(event["kernel_invocation_path"], "not_used");
    assert_eq!(event["ledger_appended"], false);
    assert!(event["fields"]["instance_count"].is_string());
}
